// Make the emulator not produce a terminal in windows on release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod debugger;
mod emulator;
mod input;

use eframe::egui::{self, IconData};
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

use crate::config::LaunchConfig;

fn main() -> eframe::Result {
    // Making sure the log guard doesn't fall out of scope
    let _log_guard = init_logging("logs", "psx.log");

    let launch_config = LaunchConfig::build().unwrap_or_else(|err| {
        error!(%err, "error building launch config");
        std::process::exit(1);
    });

    run_gui(launch_config)
}

fn run_gui(launch_config: LaunchConfig) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 800.0])
            .with_min_inner_size([640.0, 480.0])
            .with_icon(IconData::default()),
        ..Default::default()
    };

    eframe::run_native(
        "StarPSX",
        options,
        Box::new(move |cc| {
            // Start the ui thread
            Ok(Box::new(app::Application::new(cc, launch_config)))
        }),
    )
}

fn init_logging(dir: &str, filename: &str) -> WorkerGuard {
    // Clear the previous log file
    let path = dirs::data_local_dir()
        .expect("local data directory")
        .join("StarPSX")
        .join(dir);

    let _ = std::fs::remove_file(path.join(filename)); // ignore errors

    // Panics log to tracing now
    std::panic::set_hook(Box::new(|err| {
        error!(%err, "panic");
    }));

    // Start logging to stdout and log file
    let file_appender = tracing_appender::rolling::never(path.as_path(), filename);
    let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .without_time()
        .with_ansi(false);

    let stdout_layer = fmt::layer().without_time();

    let filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .with(filter)
        .init();

    info!(?path, "initialized logging at");

    file_guard
}
