mod app;

use std::{fs::File, io};
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};
use winit::error::EventLoopError;

fn clear_log_file(path: &str) -> Result<(), io::Error> {
    File::create(path)?;
    Ok(())
}

fn main() -> Result<(), EventLoopError> {
    clear_log_file("./logs/psx.log").expect("Could not clear log file");
    let file_appender = tracing_appender::rolling::never("./logs", "psx.log");

    let (file_writer, _file_guard) = tracing_appender::non_blocking(file_appender);
    let (stdout_writer, _stdout_guard) = tracing_appender::non_blocking(std::io::stdout());

    let file_log = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .without_time();

    let stdout_log = tracing_subscriber::fmt::layer()
        .with_writer(stdout_writer)
        .without_time()
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(file_log)
        .with(stdout_log)
        .with(EnvFilter::from_default_env())
        .init();

    let event_loop = winit::event_loop::EventLoop::new()?;
    let config = starpsx_core::Config::build().unwrap_or_else(|err| {
        error!(%err, "failed to parse command-line arguments");
        std::process::exit(1);
    });

    event_loop.run_app(&mut app::App {
        state: None,
        config: Some(config),
    })
}
