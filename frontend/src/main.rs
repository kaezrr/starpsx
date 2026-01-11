mod app;
mod utils;

use eframe::egui;
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> eframe::Result {
    if let Err(msg) = utils::initialize_logging() {
        eprintln!("Error: {msg}");
        std::process::exit(1);
    }

    let file_appender = tracing_appender::rolling::never("./logs", "psx.log");
    let (file_writer, _file_guard) = tracing_appender::non_blocking(file_appender);
    let (stdout_writer, _stdout_guard) = tracing_appender::non_blocking(std::io::stdout());

    let file_log = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_target(false)
        .with_ansi(false)
        .without_time();

    let stdout_log = tracing_subscriber::fmt::layer()
        .with_writer(stdout_writer)
        .with_target(false)
        .without_time()
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(file_log)
        .with(stdout_log)
        .with(EnvFilter::from_default_env())
        .init();

    let config = starpsx_core::Config::build().unwrap_or_else(|err| {
        error!(%err, "failed to parse command-line arguments");
        std::process::exit(1);
    });

    let system = starpsx_core::System::build(config).unwrap_or_else(|err| {
        error!(%err, "error while starting emulator");
        std::process::exit(1);
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "StarPSX",
        options,
        Box::new(|cc| Ok(Box::new(app::Application::new(cc)))),
    )
}
