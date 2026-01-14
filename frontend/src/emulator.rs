use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::time::{Duration, Instant};

use cpal::traits::StreamTrait;
use eframe::egui;
use starpsx_core::RunType;
use starpsx_renderer::FrameBuffer;
use tracing::error;

use crate::audio;
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
    audio_tx: SyncSender<[i16; 2]>,

    system: starpsx_core::System,
    shared_metrics: Arc<CoreMetrics>,

    is_paused: bool,

    audio_stream: cpal::Stream,

    // Local metrics
    fps_counter: u32,
    fps_timer: Instant,
    core_time_acc: Duration,
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
        let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<[i16; 2]>(735);

        let audio_stream = audio::build(audio_rx)?;

        let system = Emulator::build_core(bios_path, file_path)?;

        Ok(Self {
            ui_ctx,

            frame_tx,
            input_rx,
            audio_tx,

            system,
            shared_metrics,

            is_paused: false,

            audio_stream,

            fps_counter: 0,
            fps_timer: Instant::now(),
            core_time_acc: Duration::ZERO,
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
    }

    fn tick_metrics(&mut self) {
        self.fps_counter += 1;

        if self.fps_counter > 0 && self.fps_timer.elapsed().as_secs_f64() >= 0.5 {
            let fps = self.fps_counter * 2;
            let core_time_ms =
                (self.core_time_acc.as_secs_f32() / self.fps_counter as f32) * 1000.0;

            self.shared_metrics.store(fps, core_time_ms);

            self.fps_timer = Instant::now();
            self.fps_counter = 0;
            self.core_time_acc = Duration::ZERO;
        }
    }

    fn update_core_gamepad(&mut self, new_state: GamepadState) {
        let gamepad = self.system.gamepad_mut();
        gamepad.set_buttons(new_state.buttons);
        gamepad.set_analog_mode(new_state.analog_mode);
        gamepad.set_stick_axis(new_state.left_stick, new_state.right_stick);
    }

    fn main_loop(&mut self) {
        'emulator: loop {
            // Read events from ui thread
            while let Ok(command) = self.input_rx.try_recv() {
                match command {
                    UiCommand::NewInputState(gamepad_state) => {
                        self.update_core_gamepad(gamepad_state)
                    }
                    UiCommand::SetPaused(is_paused) => {
                        self.is_paused = is_paused;
                        match self.is_paused {
                            true => self.audio_stream.pause().unwrap(),
                            false => self.audio_stream.play().unwrap(),
                        }
                    }
                    // Shutdown the thread
                    UiCommand::Shutdown => {
                        let _ = self.audio_stream.pause();
                        break 'emulator;
                    }
                }
            }

            if self.is_paused {
                std::thread::sleep(Duration::from_millis(16));
                continue;
            }

            let now = Instant::now();
            let samples = self.system.tick();
            self.core_time_acc += now.elapsed();

            // Push samples to audio callback to a blocking channel
            // This is how the emulator is synced with audio
            self.audio_tx.send(samples).unwrap_or_else(|err| {
                error!(%err, "could not send sample to audio thread, exiting...");
                std::process::exit(1);
            });

            let frame_opt = self.system.produced_frame_buffer.take();

            if let Some(frame_buffer) = frame_opt {
                // Non blocking send
                let _ = self.frame_tx.try_send(frame_buffer);

                self.ui_ctx.request_repaint();
                self.tick_metrics();
            }
        }
    }
}

#[derive(Default)]
pub struct CoreMetrics {
    fps: AtomicU32,
    core_time_ms: AtomicU32,
}

impl CoreMetrics {
    fn store(&self, fps: u32, core_time_ms: f32) {
        self.fps.store(fps, Ordering::Relaxed);
        self.core_time_ms
            .store(core_time_ms.to_bits(), Ordering::Relaxed);
    }

    pub fn load(&self) -> (u32, f32) {
        let fps = self.fps.load(Ordering::Relaxed);
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
