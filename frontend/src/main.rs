mod app;
mod util;

use std::{
    sync::mpsc::{Receiver, SyncSender},
    time::Duration,
};

use eframe::egui;
use tracing::{error, info, trace, warn};
use tracing_subscriber::fmt;

use starpsx_renderer::FrameBuffer;
use util::GamepadState;

fn main() -> eframe::Result {
    fmt().without_time().init();

    let config = starpsx_core::Config::build().unwrap_or_else(|err| {
        error!(%err, "Failed to parse command-line arguments");
        std::process::exit(1);
    });

    let system = starpsx_core::System::build(config).unwrap_or_else(|err| {
        error!(%err, "Error while starting emulator");
        std::process::exit(1);
    });

    let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel::<FrameBuffer>(1);
    let (input_tx, input_rx) = std::sync::mpsc::sync_channel::<GamepadState>(1);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "StarPSX",
        options,
        // This must only be called once!
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || run_core(ctx, frame_tx, input_rx, system));
            Ok(Box::new(app::Application::new(cc, frame_rx, input_tx)))
        }),
    )
}

fn run_core(
    repaint_notifier: egui::Context,
    frame_tx: SyncSender<FrameBuffer>,
    input_rx: Receiver<GamepadState>,
    mut system: starpsx_core::System,
) {
    loop {
        if let Ok(input_state) = input_rx.try_recv() {
            let gamepad = system.gamepad_mut();
            gamepad.set_buttons(input_state.buttons);
            gamepad.set_analog_mode(input_state.analog_mode);
            gamepad.set_stick_axis(input_state.left_stick, input_state.right_stick);
        }

        system.step_frame();
        let Some(frame_buffer) = system.produced_frame_buffer.take() else {
            warn!("core did not produce a frame buffer");
            continue;
        };

        if frame_tx.try_send(frame_buffer).is_ok() {
            repaint_notifier.request_repaint();
        }

        std::thread::sleep(Duration::from_millis(16));
    }
}
