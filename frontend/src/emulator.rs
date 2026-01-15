use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::time::{Duration, Instant};

use eframe::egui;
use starpsx_core::RunType;
use starpsx_renderer::FrameBuffer;
use tracing::info;

use crate::config::RunnablePath;
use crate::input::GamepadState;

pub enum UiCommand {
    NewInputState(GamepadState),
    SetPaused(bool),
    Shutdown,
}

pub struct Emulator {
    ui_ctx: egui::Context,

    frame_tx: SyncSender<FrameBuffer>,
    input_rx: Receiver<UiCommand>,

    system: starpsx_core::System,
    shared_metrics: Arc<CoreMetrics>,

    is_paused: bool,
}

impl Emulator {
    pub fn build(
        ui_ctx: egui::Context,

        frame_tx: SyncSender<FrameBuffer>,
        input_rx: Receiver<UiCommand>,
        shared_metrics: Arc<CoreMetrics>,

        bios_path: &Path,
        file_path: &Option<RunnablePath>,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            ui_ctx,

            frame_tx,
            input_rx,

            system: Emulator::build_core(bios_path, file_path)?,
            shared_metrics,

            is_paused: false,
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

    fn update_core_gamepad(&mut self, new_state: GamepadState) {
        let gamepad = self.system.gamepad_mut();
        gamepad.set_buttons(new_state.buttons);
        gamepad.set_analog_mode(new_state.analog_mode);
        gamepad.set_stick_axis(new_state.left_stick, new_state.right_stick);
    }

    fn main_loop(&mut self) {
        const FRAME_TIME: Duration = Duration::from_nanos(16_666_667);

        'emulator: loop {
            // Read events from ui thread
            while let Ok(command) = self.input_rx.try_recv() {
                match command {
                    UiCommand::NewInputState(gamepad_state) => {
                        self.update_core_gamepad(gamepad_state)
                    }
                    UiCommand::SetPaused(is_paused) => {
                        self.is_paused = is_paused;
                    }
                    // Shutdown the thread
                    UiCommand::Shutdown => {
                        break 'emulator;
                    }
                }
            }

            if self.is_paused {
                std::thread::sleep(Duration::from_millis(16));
                continue;
            }

            let now = Instant::now();
            let frame_buffer = self.system.run_frame();
            let core_time = now.elapsed();

            // Non blocking send
            let _ = self.frame_tx.try_send(frame_buffer);
            self.ui_ctx.request_repaint();

            if let Some(sleep_dur) = FRAME_TIME.checked_sub(core_time) {
                std::thread::sleep(sleep_dur);
            }

            let core_time = core_time.as_secs_f32();
            let frame_time = now.elapsed().as_secs_f32();

            self.shared_metrics.store(frame_time, core_time);
        }
        info!("emulator thread stopped!");
    }
}

#[derive(Default)]
pub struct CoreMetrics {
    frame_time_ms: AtomicU32,
    core_time_ms: AtomicU32,
}

impl CoreMetrics {
    fn store(&self, frame_time_ms: f32, core_time_ms: f32) {
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
