mod app;
mod util;

use std::{sync::mpsc::SyncSender, time::Duration};

use eframe::egui;
use starpsx_renderer::FrameBuffer;
use tracing::{error, warn};
use tracing_subscriber::fmt;

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
            std::thread::spawn(move || run_core(ctx, frame_tx, system));
            Ok(Box::new(app::Application::new(cc, frame_rx)))
        }),
    )
}

fn run_core(
    repaint_notifier: egui::Context,
    frame_tx: SyncSender<FrameBuffer>,
    mut system: starpsx_core::System,
) {
    loop {
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
