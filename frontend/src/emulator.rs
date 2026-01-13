use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::time::{Duration, Instant};

use eframe::egui;
use starpsx_renderer::FrameBuffer;
use tracing::error;

use crate::input::GamepadState;

pub struct Emulator {
    ui_ctx: egui::Context,

    frame_tx: SyncSender<FrameBuffer>,
    input_rx: Receiver<GamepadState>,
    audio_tx: SyncSender<[i16; 2]>,

    system: starpsx_core::System,
    shared_metrics: Arc<CoreMetrics>,

    // Local metrics
    fps_counter: u32,
    fps_timer: Instant,
    core_time_acc: Duration,
}

impl Emulator {
    pub fn build(
        config: starpsx_core::Config,
        ui_ctx: egui::Context,
        frame_tx: SyncSender<FrameBuffer>,
        input_rx: Receiver<GamepadState>,
        audio_tx: SyncSender<[i16; 2]>,
        shared_metrics: Arc<CoreMetrics>,
    ) -> Result<Self, Box<dyn Error>> {
        let system = starpsx_core::System::build(config)?;

        Ok(Self {
            ui_ctx,

            frame_tx,
            input_rx,
            audio_tx,

            system,
            shared_metrics,

            fps_counter: 0,
            fps_timer: Instant::now(),
            core_time_acc: Duration::ZERO,
        })
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

    fn main_loop(&mut self) -> ! {
        loop {
            // Read input events from ui thread
            while let Ok(input_state) = self.input_rx.try_recv() {
                let gamepad = self.system.gamepad_mut();
                gamepad.set_buttons(input_state.buttons);
                gamepad.set_analog_mode(input_state.analog_mode);
                gamepad.set_stick_axis(input_state.left_stick, input_state.right_stick);
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
