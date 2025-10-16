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
use std::error::Error;
use timers::Timers;

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

    tty: String,
    scheduler: EventScheduler,
}

impl System {
    pub fn build(config: Config) -> Result<Self, Box<dyn Error>> {
        let cpu = Cpu::default();
        let scheduler = EventScheduler::default();
        let dma = DMAController::default();
        let gpu = Gpu::default();
        let irqctl = InterruptController::default();
        let timers = Timers::default();
        let bios = Bios::build(&config.bios_path)?;
        let ram = Ram::default();
        let scratch = Scratch::default();
        let cdrom = CdRom::default();

        let mut psx = System {
            cpu,
            gpu,
            ram,
            bios,
            scratch,
            dma,
            timers,
            irqctl,
            tty: String::new(),
            scheduler,
            cdrom,
        };

        if let Some(exe_path) = config.exe_path {
            psx.sideload_exe(&exe_path)?;
        }

        // Schedule some initial events
        psx.scheduler
            .subscribe(Event::VBlankStart, LINE_DURATION * 240, None);
        psx.scheduler
            .subscribe(Event::HBlankStart, LINE_DURATION - HBLANK_DURATION, None);

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
            let cycles = self.scheduler.cycles_till_next_event();

            for _ in (0..cycles).step_by(2) {
                Cpu::run_instruction(self);
                self.check_for_tty_output();
                self.scheduler.step();
            }

            match self.scheduler.get_next_event() {
                Event::VBlankStart => {
                    Timers::enter_vsync(self);
                    self.irqctl.stat().set_vblank(true);
                    self.scheduler
                        .subscribe(Event::VBlankEnd, LINE_DURATION * 23, None);
                }
                Event::VBlankEnd => {
                    Timers::exit_vsync(self);
                    self.gpu.renderer.copy_display_to_fb();
                    self.scheduler
                        .subscribe(Event::VBlankStart, LINE_DURATION * 240, None);
                    return;
                }
                Event::HBlankStart => {
                    Timers::enter_hsync(self);
                    self.scheduler
                        .subscribe(Event::HBlankEnd, HBLANK_DURATION, None);
                }
                Event::HBlankEnd => {
                    Timers::exit_hsync(self);
                    self.scheduler.subscribe(
                        Event::HBlankStart,
                        LINE_DURATION - HBLANK_DURATION,
                        None,
                    );
                }
                Event::Timer(x) => {
                    Timers::process_interrupt(self, x);
                }
            }
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

    fn check_for_tty_output(&mut self) {
        let pc = self.cpu.pc & 0x1FFFFFFF;
        if (pc == 0xA0 && self.cpu.regs[9] == 0x3C) || (pc == 0xB0 && self.cpu.regs[9] == 0x3D) {
            let ch = self.cpu.regs[4] as u8 as char;
            self.tty.push(ch);
            if ch == '\n' || ch == '\r' {
                print!("{}", self.tty);
                self.tty = String::new();
            }
        }
    }
}
