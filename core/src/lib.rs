mod cdrom;
mod consts;
mod cpu;
mod dma;
mod gpu;
mod irq;
mod mem;
mod sched;
mod sio;
mod spu;
mod timers;

use cdrom::CdImage;
use cdrom::CdRom;
use consts::{HBLANK_DURATION, LINE_DURATION};
use cpu::Cpu;
use dma::DMAController;
use gpu::Gpu;
use irq::InterruptController;
use mem::bios::Bios;
use mem::ram::Ram;
use mem::scratch::Scratch;
use sched::{Event, EventScheduler};
use sio::Sio0;
use starpsx_renderer::FrameBuffer;
use std::error::Error;
use timers::Timers;
use tracing::info;

pub use sio::gamepad;

use crate::sio::Sio1;
use crate::spu::Spu;

pub enum RunType {
    Game(Vec<u8>),
    Executable(Vec<u8>),
}

pub struct System {
    cpu: Cpu,
    gpu: Gpu,
    spu: Spu,

    ram: Ram,
    bios: Bios,
    scratch: Scratch,

    dma: DMAController,
    timers: Timers,
    irqctl: InterruptController,

    cdrom: CdRom,

    sio0: Sio0,
    sio1: Sio1,

    tty: String,
    scheduler: EventScheduler,

    // RGBA frame buffer
    pub produced_frame_buffer: Option<FrameBuffer>,
}

impl System {
    pub fn build(bios: Vec<u8>, runnable: Option<RunType>) -> Result<Self, Box<dyn Error>> {
        let mut psx = System {
            cpu: Cpu::default(),
            gpu: Gpu::default(),
            spu: Spu::default(),

            ram: Ram::default(),
            bios: Bios::new(bios)?,
            scratch: Scratch::default(),

            dma: DMAController::default(),
            timers: Timers::default(),
            irqctl: InterruptController::default(),

            tty: String::new(),
            scheduler: EventScheduler::default(),

            cdrom: CdRom::default(),
            // Only 1 gamepad for now
            sio0: Sio0::new([Some(gamepad::Gamepad::default()), None]),
            sio1: Sio1 {}, // Does nothing

            produced_frame_buffer: None,
        };

        // Load game or exe
        if let Some(run_type) = runnable {
            // Do not open the shell after bios start
            psx.cdrom.status.set_shell_open(false);
            match run_type {
                RunType::Game(bytes) => psx.cdrom.insert_disc(CdImage::from_bytes(bytes)),
                RunType::Executable(bytes) => psx.sideload_exe(bytes),
            }
        } else {
            psx.cdrom.status.set_shell_open(true);
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

    // Run emulator for one frame and return the generated frame
    pub fn run_frame<const SHOW_VRAM: bool>(&mut self) -> FrameBuffer {
        loop {
            if let Some(event) = self.scheduler.get_next_event() {
                match event {
                    // Frame completes just before entering vsync
                    Event::VBlankStart => return self.enter_vsync::<SHOW_VRAM>(),
                    Event::VBlankEnd => Timers::exit_vsync(self),
                    Event::HBlankStart => Timers::enter_hsync(self),
                    Event::HBlankEnd => Timers::exit_hsync(self),
                    Event::Timer(x) => Timers::process_interrupt(self, x),
                    Event::SerialSend => Sio0::process_serial_send(self),
                    Event::CdromResultIrq(x) => CdRom::handle_response(self, x),
                }
            }

            // Fixed 2 CPI right now
            Cpu::run_instruction(self);
            self.scheduler.advance(2);

            self.check_for_tty_output();
        }
    }

    pub fn sideload_exe(&mut self, exe: Vec<u8>) {
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
    }

    fn enter_vsync<const SHOW_VRAM: bool>(&mut self) -> FrameBuffer {
        Timers::enter_vsync(self);
        self.irqctl.stat().set_vblank(true);
        self.gpu.renderer.produce_frame_buffer::<SHOW_VRAM>()
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
        self.sio0.gamepad_port_0_mut()
    }

    pub fn snapshot(&self) -> SystemSnapshot {
        let cpu = self.cpu.snapshot();

        let base = cpu.pc.wrapping_sub(100 * 4);
        let ins = std::array::from_fn(|i| {
            let addr = base.wrapping_add((i * 4) as u32);
            let inst = self.fetch_instruction(addr);
            (addr, inst)
        });

        SystemSnapshot { cpu, ins }
    }
}

pub struct SystemSnapshot {
    pub cpu: cpu::Snapshot,

    /// cpu.pc +- 100
    pub ins: [(u32, u32); 200],
}
