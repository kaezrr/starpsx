use eframe::egui;
use starpsx_core::gpu::renderer::{CANVAS_HEIGHT, CANVAS_WIDTH};
use starpsx_core::{Config, StarPSX};

pub struct App {
    psx: StarPSX,
    texture: egui::TextureHandle,
    number: u32,
}

pub const SCREEN_HEIGHT: f32 = CANVAS_HEIGHT as f32 + 100.;
pub const SCREEN_WIDTH: f32 = CANVAS_WIDTH as f32 + 100.;

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::build().expect("Failed to parse config");
        let psx = StarPSX::build(config).expect("Failed to init StarPSX");
        Self {
            psx,
            texture: cc.egui_ctx.load_texture(
                "psx-img",
                egui::ColorImage::from_rgb(
                    [CANVAS_WIDTH, CANVAS_HEIGHT],
                    &vec![0; CANVAS_WIDTH * CANVAS_HEIGHT * 3],
                ),
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    minification: egui::TextureFilter::Nearest,
                    ..Default::default()
                },
            ),
            number: 0,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                    }
                })
            })
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("StarPSX {}", self.number));
            self.number += 1;
            ui.centered_and_justified(|ui| {
                self.texture.set(
                    egui::ColorImage::from_rgb(
                        [CANVAS_WIDTH, CANVAS_HEIGHT],
                        self.psx.pixel_buffer(),
                    ),
                    egui::TextureOptions {
                        magnification: egui::TextureFilter::Nearest,
                        minification: egui::TextureFilter::Nearest,
                        ..Default::default()
                    },
                );

                ui.add(
                    egui::Image::from_texture(&self.texture)
                        .maintain_aspect_ratio(true)
                        .shrink_to_fit(),
                );
            });
        });

        ctx.request_repaint();
    }
}
