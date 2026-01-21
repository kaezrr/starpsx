use std::collections::HashSet;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::time::{Duration, Instant};

use eframe::egui;
use starpsx_core::RunType;
use starpsx_renderer::FrameBuffer;
use tracing::{info, warn};

use crate::config::RunnablePath;
use crate::debugger::snapshot::DebugSnapshot;
use crate::input::GamepadState;

pub enum UiCommand {
    NewInputState(GamepadState),
    SetVramDisplay(bool),
    Restart,
    Shutdown,

    DebugSetBreakpoint(u32, bool),
    DebugStep,
    DebugRequestState,
}

pub struct Emulator {
    ui_ctx: egui::Context,
    channels: UiChannels,

    system: starpsx_core::System,
    shared_state: Arc<SharedState>,

    breakpoints: HashSet<u32>,

    bios_path: PathBuf,
    file_path: Option<RunnablePath>,

    show_vram: bool,
}

pub struct UiChannels {
    pub frame_tx: SyncSender<FrameBuffer>,
    pub input_rx: Receiver<UiCommand>,
    pub snapshot_tx: SyncSender<DebugSnapshot>,
}

impl Emulator {
    pub fn build(
        ui_ctx: egui::Context,

        channels: UiChannels,
        shared_state: Arc<SharedState>,

        bios_path: PathBuf,
        file_path: Option<RunnablePath>,

        show_vram: bool,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            ui_ctx,
            channels,
            shared_state,

            system: Emulator::build_core(&bios_path, &file_path)?,

            bios_path,
            file_path,

            breakpoints: HashSet::new(),
            show_vram,
        })
    }

    pub fn build_core(
        bios_path: &Path,
        file_path: &Option<RunnablePath>,
    ) -> Result<starpsx_core::System, Box<dyn Error>> {
        let bios = std::fs::read(bios_path)?;

        let run_type = file_path
            .as_ref()
            .map(|run_type| -> Result<RunType, io::Error> {
                let bytes = match run_type {
                    RunnablePath::Exe(path) => RunType::Executable(std::fs::read(path)?),
                    RunnablePath::Bin(path) => RunType::Game(std::fs::read(path)?),
                };
                Ok(bytes)
            })
            .transpose()?;

        starpsx_core::System::build(bios, run_type)
    }

    pub fn run(mut self) {
        std::thread::spawn(move || self.main_loop());
        info!("emulator thread started...");
    }

    fn send_debug_snapshot(&self) {
        let system_snapshot = self.system.snapshot();
        let _ = self.channels.snapshot_tx.try_send(DebugSnapshot {
            pc: system_snapshot.cpu.pc,
            lo: system_snapshot.cpu.lo,
            hi: system_snapshot.cpu.hi,
            cpu_regs: system_snapshot.cpu.regs,
            instructions: system_snapshot.ins,
        });
        info!("sending snapshot");
        self.ui_ctx.request_repaint();
    }

    fn update_core_gamepad(&mut self, new_state: GamepadState) {
        let gamepad = self.system.gamepad_mut();
        gamepad.set_buttons(new_state.buttons);
        gamepad.set_analog_mode(new_state.analog_mode);
        gamepad.set_stick_axis(new_state.left_stick, new_state.right_stick);
    }

    fn send_frame_buffer(&mut self, buffer: FrameBuffer) {
        // Non blocking send
        let _ = self.channels.frame_tx.try_send(buffer);
        self.ui_ctx.request_repaint();
    }

    fn main_loop(&mut self) {
        const FRAME_TIME: Duration = Duration::from_nanos(16_666_667);

        'emulator: loop {
            // Read events from ui thread
            while let Ok(command) = self.channels.input_rx.try_recv() {
                match command {
                    UiCommand::NewInputState(gamepad_state) => {
                        self.update_core_gamepad(gamepad_state)
                    }

                    UiCommand::SetVramDisplay(show_vram) => {
                        self.show_vram = show_vram;
                    }

                    UiCommand::Restart => {
                        self.system =
                            Emulator::build_core(&self.bios_path, &self.file_path).unwrap();
                        self.shared_state.resume();
                    }

                    UiCommand::Shutdown => {
                        break 'emulator;
                    }

                    UiCommand::DebugRequestState => {
                        self.send_debug_snapshot();
                    }

                    UiCommand::DebugSetBreakpoint(address, enabled) => {
                        match enabled {
                            true => self.breakpoints.insert(address),
                            false => self.breakpoints.remove(&address),
                        };
                    }

                    UiCommand::DebugStep => {
                        if !self.shared_state.is_paused() {
                            warn!("trying to step while emulator is unpaused");
                            continue;
                        }

                        if let Some(fb) = self.system.step_instruction(self.show_vram) {
                            self.send_frame_buffer(fb);
                        }
                        self.send_debug_snapshot();
                    }
                }
            }

            if self.shared_state.is_paused() {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }

            let now = Instant::now();
            let vram = self.show_vram;

            let frame_opt = if self.breakpoints.is_empty() {
                Some(self.system.run_frame(vram))
            } else {
                self.system.run_breakpoint(&self.breakpoints, vram)
            };

            let core_time = now.elapsed();

            if let Some(buffer) = frame_opt {
                self.send_frame_buffer(buffer);
            } else {
                self.shared_state.pause();
                self.send_debug_snapshot();
                continue;
            }

            if let Some(sleep_dur) = FRAME_TIME.checked_sub(core_time) {
                std::thread::sleep(sleep_dur);
            }

            let core_time = core_time.as_secs_f32();
            let frame_time = now.elapsed().as_secs_f32();

            self.shared_state.store(frame_time, core_time);
        }
        info!("emulator thread stopped!");
    }
}

#[derive(Default)]
pub struct SharedState {
    frame_time_ms: AtomicU32,
    core_time_ms: AtomicU32,
    is_paused: AtomicBool,
}

impl SharedState {
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::Relaxed)
    }

    pub fn store(&self, frame_time_ms: f32, core_time_ms: f32) {
        self.frame_time_ms
            .store(frame_time_ms.to_bits(), Ordering::Relaxed);
        self.core_time_ms
            .store(core_time_ms.to_bits(), Ordering::Relaxed);
    }

    pub fn load(&self) -> (f32, f32) {
        let fps = f32::from_bits(self.frame_time_ms.load(Ordering::Relaxed));
        let core_time_ms = f32::from_bits(self.core_time_ms.load(Ordering::Relaxed));
        (fps, core_time_ms)
    }
}

pub fn parse_runnable(path: PathBuf) -> Result<RunnablePath, Box<dyn Error>> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("exe") => Ok(RunnablePath::Exe(path)),
        Some("bin") => Ok(RunnablePath::Bin(path)),
        _ => Err("unsupported file format".into()),
    }
}
