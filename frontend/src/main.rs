mod app;
mod egui_tools;
mod gamepad;

use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};
use winit::event_loop::{ControlFlow, EventLoop};

// Logs to a fixed path for now
fn initialize_logging() -> Result<(), String> {
    std::fs::create_dir_all("./logs")
        .map_err(|e| format!("failed to create logs directory: {e}"))?;
    std::fs::File::create("./logs/psx.log")
        .map_err(|e| format!("failed to initialize log file: {e}"))?;
    Ok(())
}

fn main() {
    if let Err(msg) = initialize_logging() {
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

    pollster::block_on(run(system))
}

async fn run(system: starpsx_core::System) {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = app::App::new(system);
    event_loop.run_app(&mut app).expect("Failed to run app");
}
