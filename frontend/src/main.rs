// Make the emulator not produce a terminal in windows on release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod emulator;
mod input;

use eframe::egui;
use starpsx_renderer::FrameBuffer;
use tracing::error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

use input::GamepadState;

fn main() -> eframe::Result {
    // Make sure the log guard has static lifetime
    let _log_guard = init_logging("logs", "psx.log");

    // Message channels for thread communication
    let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel::<FrameBuffer>(1);
    let (input_tx, input_rx) = std::sync::mpsc::sync_channel::<GamepadState>(1);
    let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<i16>(100);

    let config = starpsx_core::Config::build().unwrap_or_else(|err| {
        error!(%err, "Failed to parse command-line arguments");
        std::process::exit(1);
    });

    let stream = audio::build_audio(audio_rx).unwrap_or_else(|err| {
        error!(?err, "error while building audio stream");
        std::process::exit(1);
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_resizable(false),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "StarPSX",
        options,
        // This must only be called once!
        Box::new(move |cc| {
            // Build emulator from the provided configuration
            let emulator = emulator::Emulator::build(
                config,
                cc.egui_ctx.clone(),
                frame_tx,
                input_rx,
                audio_tx,
            )
            .unwrap_or_else(|err| {
                error!(%err, "Error while starting emulator");
                std::process::exit(1);
            });

            // Start the emulator thread
            emulator.run();

            // Start the ui thread
            Ok(Box::new(app::Application::new(
                cc, frame_rx, input_tx, stream,
            )))
        }),
    )
}

fn init_logging(dir: &str, filename: &str) -> WorkerGuard {
    // Clear the previous log file
    let path = std::path::Path::new(dir).join(filename);
    let _ = std::fs::remove_file(path); // ignore errors

    // Panics log to tracing now
    std::panic::set_hook(Box::new(|err| {
        error!(%err, "panic");
    }));

    // Start logging to stdout and log file
    let file_appender = tracing_appender::rolling::never("logs", "psx.log");
    let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .without_time()
        .with_ansi(false);

    let stdout_layer = fmt::layer().without_time();

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .with(EnvFilter::from_default_env())
        .init();

    file_guard
}
