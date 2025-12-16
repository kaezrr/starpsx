mod cdrom;
mod cpu;
mod dma;
mod gpu;
mod irq;
mod mem;
mod sched;
mod sio;
mod timers;

use cdrom::CdRom;
use cpu::Cpu;
use dma::DMAController;
use gpu::Gpu;
use irq::InterruptController;
use mem::bios::Bios;
use mem::ram::Ram;
use mem::scratch::Scratch;
use sched::{Event, EventScheduler};
use sio::SerialInterface;
use std::error::Error;
use timers::Timers;
use tracing::info;

pub use sio::gamepad;

pub const TARGET_FPS: u64 = 60;
pub const LINE_DURATION: u64 = 2172;
pub const HBLANK_DURATION: u64 = 390;

pub struct Config {
    bios_path: String,
    exe_path: Option<String>,
}

impl Config {
    pub fn build() -> Result<Config, Box<dyn Error>> {
        let args: Vec<String> = std::env::args().collect();

        let bios_path = match args.get(1) {
            Some(x) => x.clone(),
            None => return Err("missing bios path".into()),
        };
        let exe_path = args.get(2).cloned();

        Ok(Config {
            bios_path,
            exe_path,
        })
    }
}

pub struct System {
    cpu: Cpu,
    gpu: Gpu,

    ram: Ram,
    bios: Bios,
    scratch: Scratch,

    dma: DMAController,
    timers: Timers,
    irqctl: InterruptController,

    cdrom: CdRom,
    sio: SerialInterface,

    tty: String,
    scheduler: EventScheduler,
}

impl System {
    pub fn build(config: Config) -> Result<Self, Box<dyn Error>> {
        let mut psx = System {
            cpu: Cpu::default(),
            gpu: Gpu::default(),
            ram: Ram::default(),
            bios: Bios::build(&config.bios_path)?,
            scratch: Scratch::default(),
            dma: DMAController::default(),
            timers: Timers::default(),
            irqctl: InterruptController::default(),
            tty: String::new(),
            scheduler: EventScheduler::default(),
            cdrom: CdRom::default(),

            // Only 1 gamepad for now
            sio: SerialInterface::new([Some(gamepad::Gamepad::default()), None]),
        };

        if let Some(exe_path) = config.exe_path {
            psx.sideload_exe(&exe_path)?;
        }

        // Schedule some initial events
        psx.scheduler.schedule(
            Event::VBlankStart,
            LINE_DURATION * 240,
            Some(LINE_DURATION * 263),
        );

        psx.scheduler.schedule(
            Event::VBlankEnd,
            LINE_DURATION * 263,
            Some(LINE_DURATION * 263),
        );

        psx.scheduler.schedule(
            Event::HBlankStart,
            LINE_DURATION - HBLANK_DURATION,
            Some(LINE_DURATION),
        );

        psx.scheduler
            .schedule(Event::HBlankEnd, LINE_DURATION, Some(LINE_DURATION));

        Ok(psx)
    }

    pub fn frame_buffer(&self) -> &[u32] {
        let (width, height) = self.gpu.get_resolution();
        &self.gpu.renderer.frame_buffer()[..(width * height)]
    }

    pub fn frame_buffer_vram(&self) -> &[u32] {
        self.gpu.renderer.frame_buffer()
    }

    pub fn get_resolution(&self) -> (u32, u32) {
        let (width, height) = self.gpu.get_resolution();
        (width as u32, height as u32)
    }

    pub fn step_frame(&mut self) {
        loop {
            if let Some(event) = self.scheduler.get_next_event() {
                match event {
                    Event::VBlankStart => self.enter_vsync(),
                    Event::VBlankEnd => {
                        self.exit_vsync();
                        return; // end of frame
                    }
                    Event::HBlankStart => Timers::enter_hsync(self),
                    Event::HBlankEnd => Timers::exit_hsync(self),
                    Event::Timer(x) => Timers::process_interrupt(self, x),
                    Event::SerialSend => SerialInterface::process_serial_send(self),
                    Event::CdromResultIrq => CdRom::process_irq(self),
                }
                continue;
            }

            // Fixed 2 CPI right now
            Cpu::run_instruction(self);
            self.scheduler.advance(2);

            self.check_for_tty_output();
        }
    }

    pub fn sideload_exe(&mut self, filepath: &String) -> Result<(), Box<dyn Error>> {
        let exe = std::fs::read(filepath)?;
        while self.cpu.pc != 0x80030000 {
            Cpu::run_instruction(self);
            self.check_for_tty_output();
        }

        // Parse EXE header
        let init_pc = u32::from_le_bytes(exe[0x10..0x14].try_into().unwrap());
        let init_r28 = u32::from_le_bytes(exe[0x14..0x18].try_into().unwrap());
        let exe_addr = u32::from_le_bytes(exe[0x18..0x1C].try_into().unwrap()) & 0x1FFFFF;
        let exe_size = u32::from_le_bytes(exe[0x1C..0x20].try_into().unwrap());
        let init_sp = u32::from_le_bytes(exe[0x30..0x34].try_into().unwrap());

        // Copy EXE data to RAM
        self.ram.bytes[exe_addr as usize..(exe_addr + exe_size) as usize]
            .copy_from_slice(&exe[2048..2048 + exe_size as usize]);

        self.cpu.regs[28] = init_r28;
        if init_sp != 0 {
            self.cpu.regs[29] = init_sp;
            self.cpu.regs[30] = init_sp;
        }

        self.cpu.pc = init_pc;

        // Pass args to amidogs exe
        // let args = ["auto\0", "console\0", "release\0"];
        // let arg_len: u32 = 2; // only first 2 args
        // let mut len: usize = 0;
        //
        // for i in 0..arg_len {
        //     // write pointer to the string
        //     self.bus
        //         .write::<u32>(0x1f800004 + i * 4, 0x1f800044 + len as u32);
        //
        //     let s = args[i as usize];
        //     let n = s.len();
        //
        //     for x in len..len + n {
        //         let byte = s.as_bytes()[x - len];
        //         self.bus.write::<u8>(0x1f800044 + x as u32, byte);
        //     }
        //
        //     len += n;
        // }
        //
        // self.bus.write::<u32>(0x1f800000, arg_len);

        Ok(())
    }

    fn enter_vsync(&mut self) {
        Timers::enter_vsync(self);
        self.irqctl.stat().set_vblank(true);
    }

    fn exit_vsync(&mut self) {
        Timers::exit_vsync(self);
        self.gpu.renderer.copy_display_to_fb();
    }

    fn check_for_tty_output(&mut self) {
        let pc = self.cpu.pc & 0x1FFFFFFF;
        if (pc == 0xA0 && self.cpu.regs[9] == 0x3C) || (pc == 0xB0 && self.cpu.regs[9] == 0x3D) {
            let ch = self.cpu.regs[4] as u8 as char;
            if ch == '\n' || ch == '\r' {
                info!("[TTY]" = %self.tty);
                self.tty = String::new();
            } else {
                self.tty.push(ch);
            }
        }
    }

    pub fn gamepad_mut(&mut self) -> &mut gamepad::Gamepad {
        self.sio.gamepad_port_0_mut()
    }
}
