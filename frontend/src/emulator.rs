use std::error::Error;
use std::sync::mpsc::{Receiver, SyncSender};

use eframe::egui;
use starpsx_renderer::FrameBuffer;
use tracing::error;

use crate::input::GamepadState;

pub struct Emulator {
    ui_ctx: egui::Context,
    frame_tx: SyncSender<FrameBuffer>,
    input_rx: Receiver<GamepadState>,
    audio_tx: SyncSender<i16>,
    system: starpsx_core::System,
}

impl Emulator {
    pub fn build(
        config: starpsx_core::Config,
        ui_ctx: egui::Context,
        frame_tx: SyncSender<FrameBuffer>,
        input_rx: Receiver<GamepadState>,
        audio_tx: SyncSender<i16>,
    ) -> Result<Self, Box<dyn Error>> {
        let system = starpsx_core::System::build(config)?;

        Ok(Self {
            ui_ctx,
            frame_tx,
            input_rx,
            audio_tx,
            system,
        })
    }

    pub fn run(mut self) {
        std::thread::spawn(move || self.main_loop());
    }

    fn main_loop(&mut self) -> ! {
        loop {
            while let Ok(input_state) = self.input_rx.try_recv() {
                let gamepad = self.system.gamepad_mut();
                gamepad.set_buttons(input_state.buttons);
                gamepad.set_analog_mode(input_state.analog_mode);
                gamepad.set_stick_axis(input_state.left_stick, input_state.right_stick);
            }

            for sample in self.system.tick() {
                self.audio_tx.send(sample).unwrap_or_else(|err| {
                    error!(%err, "could not send sample to audio thread, exiting...");
                    std::process::exit(1);
                });
            }

            let frame_sent = self
                .system
                .produced_frame_buffer
                .take()
                .map(|buf| self.frame_tx.try_send(buf).is_ok())
                .unwrap_or(false);

            if frame_sent {
                self.ui_ctx.request_repaint();
            };
        }
    }
}
