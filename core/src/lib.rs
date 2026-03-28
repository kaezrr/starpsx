mod cdrom;
mod consts;
mod cpu;
mod dma;
mod gpu;
mod irq;
mod mdec;
mod mem;
mod sched;
mod sio;
mod spu;
mod timers;

use std::collections::HashSet;

use anyhow::Context;
use cdrom::CdImage;
use cdrom::CdRom;
use consts::HBLANK_DURATION;
use consts::LINE_DURATION;
use cpu::Cpu;
use dma::DMAController;
use gpu::Gpu;
pub use gpu::Snapshot as GpuSnapshot;
pub use gpu::VMode;
use irq::InterruptController;
use mem::bios::Bios;
use mem::ram::Ram;
use mem::scratch::Scratch;
use sched::Event;
use sched::EventScheduler;
use sio::Sio0;
pub use sio::gamepad;
pub use spu::AdsrPhase;
pub use spu::Snapshot as SpuSnapshot;
pub use spu::VoiceSnapshot;
use starpsx_renderer::FrameBuffer;
use timers::Timers;
use tracing::info;

use crate::mdec::MacroDecoder;
use crate::sio::Sio1;
use crate::sio::gamepad::Gamepad;
use crate::sio::memory_card::MemoryCard;
use crate::spu::Spu;

pub enum RunType {
    Disk(cue::CdDisk),
    Binary(Vec<u8>),
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
    mdec: MacroDecoder,

    sio0: Sio0,
    sio1: Sio1,

    tty: Vec<u8>,
    scheduler: EventScheduler,

    // RGBA frame buffer
    pub frame_buffer: Option<FrameBuffer>,
}

impl System {
    /// # Errors
    ///
    /// This function will return an error if:
    /// * Bios file is not valid
    /// * Runnable file is not valid
    pub fn build(
        bios: Vec<u8>,
        runnable: Option<RunType>,
        memory_card: Option<Box<[u8; 0x20000]>>,
    ) -> anyhow::Result<Self> {
        let mut psx = Self {
            cpu: Cpu::default(),
            gpu: Gpu::default(),
            spu: Spu::default(),

            ram: Ram::default(),
            bios: Bios::new(bios)?,
            scratch: Scratch::default(),

            dma: DMAController::default(),
            timers: Timers::default(),
            irqctl: InterruptController::default(),
            cdrom: CdRom::default(),
            mdec: MacroDecoder::default(),

            tty: Vec::new(),
            scheduler: EventScheduler::default(),

            // Only 1 gamepad and memory card  for now
            sio0: Sio0::new(
                [Some(Gamepad::default()), None],
                [memory_card.map(MemoryCard::from_bytes), None],
            ),
            sio1: Sio1, // Does nothing

            frame_buffer: None,
        };

        // Load game or exe
        if let Some(run_type) = runnable {
            // Do not open the shell after bios start
            match run_type {
                RunType::Disk(disk) => psx.cdrom.insert_disc(CdImage::from_disk(disk)),
                RunType::Binary(bytes) => psx.cdrom.insert_disc(CdImage::from_bytes(bytes)),
                RunType::Executable(bytes) => psx.sideload_exe(&bytes)?,
            }
        } else {
            psx.cdrom.open_shell();
        }

        // // -------------------CDROM TESTING ONLY-----------------------
        // psx.cdrom.insert_disc(CdImage::from_disk(
        //     cue::build_disk("../dev/pcsx_redux_cdrom_tests/test.cue").unwrap(),
        // ));
        // // ------------------------------------------------------------

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

        psx.scheduler.schedule(Event::SpuTick, 768, Some(768));

        Ok(psx)
    }

    fn sideload_exe(&mut self, exe: &[u8]) -> anyhow::Result<()> {
        while self.cpu.pc != 0x8003_0000 {
            Cpu::run_next_instruction(self);
            self.check_for_tty_output();
        }

        // Wrap header reads in a helper to avoid repetitive boilerplate
        let rd = |off| -> anyhow::Result<u32> {
            exe.get(off..off + 4)
                .and_then(|b| b.try_into().ok())
                .map(u32::from_le_bytes)
                .context("Invalid PS-EXE header")
        };

        let init_pc = rd(0x10)?;
        let init_r28 = rd(0x14)?;
        let exe_addr = rd(0x18)? & 0x001F_FFFF;
        let exe_size = rd(0x1C)? as usize;
        let init_sp = rd(0x30)?;

        // Use get_mut to avoid the slice panic if exe_size is garbage
        let dest = self
            .ram
            .bytes
            .get_mut(exe_addr as usize..exe_addr as usize + exe_size)
            .context("EXE load address out of RAM bounds")?;

        let src = exe
            .get(2048..2048 + exe_size)
            .context("EXE file truncated")?;

        dest.copy_from_slice(src);

        self.cpu.regs[28] = init_r28;
        if init_sp != 0 {
            self.cpu.regs[29] = init_sp;
            self.cpu.regs[30] = init_sp;
        }

        self.cpu.pc = init_pc;

        Ok(())
    }

    fn enter_vsync(&mut self, show_vram: bool) -> FrameBuffer {
        Timers::enter_vsync(self);
        self.gpu.enter_vsync();
        self.irqctl.stat().set_vblank(true);

        if show_vram {
            self.gpu.renderer.produce_vram_framebuffer()
        } else {
            self.gpu.renderer.produce_frame_buffer()
        }
    }

    fn exit_vsync(&mut self) {
        self.gpu.exit_vsync();
        Timers::exit_vsync(self);
    }

    fn enter_hsync(&mut self) {
        self.gpu.enter_hsync();
        Timers::enter_hsync(self);
    }

    fn check_for_tty_output(&mut self) {
        let pc = self.cpu.pc & 0x1FFF_FFFF;
        if (pc == 0xA0 && self.cpu.regs[9] == 0x3C) || (pc == 0xB0 && self.cpu.regs[9] == 0x3D) {
            let byte = self.cpu.regs[4] as u8;
            if byte == b'\n' || byte == b'\r' {
                info!("[TTY]" = %String::from_utf8_lossy(&self.tty));
                self.tty.clear();
            } else {
                self.tty.push(byte);
            }
        }
    }

    /// # Panics
    ///
    /// Panics if console has no virtual gamepad in port 0
    pub const fn gamepad_mut(&mut self) -> &mut Gamepad {
        self.sio0.device_manager.gamepads[0]
            .as_mut()
            .expect("atleast 1 gamepad")
    }

    pub const fn memory_card(&mut self) -> Option<&mut MemoryCard> {
        self.sio0.device_manager.memcards[0].as_mut()
    }

    #[must_use]
    pub fn snapshot(&self) -> SystemSnapshot {
        let cpu = self.cpu.snapshot();

        let base = cpu.pc.wrapping_sub(100 * 4);
        let ins = std::array::from_fn(|i| {
            let addr = base.wrapping_add((i * 4) as u32);
            let inst = self.fetch_instruction(addr);
            (addr, inst)
        });

        let spu = self.spu.snapshot();
        let gpu = self.gpu.snapshot();

        SystemSnapshot { cpu, spu, gpu, ins }
    }

    pub fn step_instruction(&mut self, show_vram: bool) -> Option<[i16; 2]> {
        if let Some(event) = self.scheduler.get_next_event() {
            match event {
                Event::VBlankStart => self.frame_buffer = Some(self.enter_vsync(show_vram)),
                Event::VBlankEnd => self.exit_vsync(),
                Event::HBlankStart => self.enter_hsync(),
                Event::HBlankEnd => Timers::exit_hsync(self),
                Event::Timer(x) => Timers::process_interrupt(self, x),
                Event::SerialSend => Sio0::process_serial_send(self),
                Event::CdromResultIrq(x) => CdRom::handle_response(self, x),
                Event::DsrOff => self.sio0.turn_off_dsr(),
                Event::SpuTick => {
                    return Some(Spu::tick(self).unwrap_or([0, 0]));
                }
            }
        }

        // Fixed 2 CPI right now
        Cpu::run_next_instruction(self);
        self.scheduler.advance(2);

        self.check_for_tty_output();
        None
    }

    pub fn run_one_spu_tick(&mut self, show_vram: bool) -> [i16; 2] {
        loop {
            if let Some(sample) = self.step_instruction(show_vram) {
                break sample;
            }
        }
    }

    // Run emulator for one frame
    pub fn run_frame(&mut self, show_vram: bool) {
        for _ in (0..565_053).step_by(2) {
            let _ = self.step_instruction(show_vram);
        }
    }

    // Run emulator until it generates a frame or hits a breakpoint
    pub fn run_till_breakpoint(&mut self, breakpoints: &HashSet<u32>) {
        loop {
            if breakpoints.contains(&self.cpu.pc) {
                return;
            }
            let _ = self.step_instruction(false);
        }
    }
}

pub struct SystemSnapshot {
    pub cpu: cpu::Snapshot,
    pub spu: spu::Snapshot,
    pub gpu: gpu::Snapshot,

    /// cpu.pc +- 100
    pub ins: [(u32, u32); 200],
}
