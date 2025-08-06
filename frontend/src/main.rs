use eframe::egui;
use starpsx::{self, App, SCREEN_HEIGHT, SCREEN_WIDTH};

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([SCREEN_WIDTH, SCREEN_HEIGHT]),
        ..Default::default()
    };
    eframe::run_native(
        "StarPSX",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
