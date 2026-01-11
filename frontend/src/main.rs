mod app;
mod utils;

use eframe::egui;
use tracing::error;
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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "StarPSX",
        options,
        Box::new(move |cc| Ok(Box::new(app::Application::new(cc, system)))),
    )
}
