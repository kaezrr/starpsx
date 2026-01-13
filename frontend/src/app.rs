use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};

use eframe::egui::{self, Color32, ColorImage};
use eframe::egui::{TextureOptions, ViewportCommand, load::SizedTexture};
use starpsx_renderer::FrameBuffer;
use tracing::{info, trace};

use crate::emulator::{CoreMetrics, UiCommand};
use crate::input::{self, ActionValue, PhysicalInput};

pub struct Application {
    gamepad: gilrs::Gilrs,
    input_state: input::GamepadState,
    keybindings: input::Bindings,

    frame_rx: Receiver<FrameBuffer>,
    input_tx: SyncSender<UiCommand>,

    texture: egui::TextureHandle,

    is_paused: bool,

    // metrics
    last_resolution: Option<(usize, usize)>,
    shared_metrics: Arc<CoreMetrics>,

    info_modal_open: bool,
}

impl Application {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        frame_rx: Receiver<FrameBuffer>,
        input_tx: SyncSender<UiCommand>,
        shared_metrics: Arc<CoreMetrics>,
    ) -> Self {
        Self {
            gamepad: gilrs::Gilrs::new().expect("could not initalize gilrs"),
            input_state: input::GamepadState::default(),
            keybindings: input::get_default_keybinds(),

            frame_rx,
            input_tx,

            texture: cc.egui_ctx.load_texture(
                "frame buffer",
                ColorImage::filled([100, 100], Color32::BLACK),
                egui::TextureOptions::NEAREST,
            ),

            is_paused: false,

            // Performance metrics
            last_resolution: None,
            shared_metrics,

            info_modal_open: false,
        }
    }

    fn process_keyboard_events(&mut self, ctx: &egui::Context) -> bool {
        if ctx.wants_keyboard_input() {
            return false;
        }

        let mut changed = false;
        ctx.input(|i| {
            for (phys, action) in &self.keybindings {
                let PhysicalInput::Key(key) = phys else {
                    continue;
                };

                if i.key_pressed(*key) {
                    changed |= self
                        .input_state
                        .handle_action(action, ActionValue::Digital(true));
                }

                if i.key_released(*key) {
                    changed |= self
                        .input_state
                        .handle_action(action, ActionValue::Digital(false));
                }
            }
        });
        changed
    }

    fn process_gamepad_events(&mut self) -> bool {
        let mut changed = false;
        while let Some(gilrs::Event { event, .. }) = self.gamepad.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(button, _) => {
                    let phys = PhysicalInput::GilrsButton(button);
                    if let Some(action) = self.keybindings.get(&phys) {
                        changed |= self
                            .input_state
                            .handle_action(action, ActionValue::Digital(true));
                    }
                }

                gilrs::EventType::ButtonReleased(button, _) => {
                    let phys = PhysicalInput::GilrsButton(button);
                    if let Some(action) = self.keybindings.get(&phys) {
                        changed |= self
                            .input_state
                            .handle_action(action, ActionValue::Digital(false));
                    }
                }

                gilrs::EventType::AxisChanged(axis, value, _) => {
                    let phys = PhysicalInput::GilrsAxis(axis);
                    if let Some(action) = self.keybindings.get(&phys) {
                        changed |= self
                            .input_state
                            .handle_action(action, ActionValue::Analog(value));
                    }
                }

                gilrs::EventType::Connected => info!("gamepad connected"),
                gilrs::EventType::Disconnected => info!("gamepad disconnected"),

                _ => trace!(?event, "gamepad event ignored"),
            }
        }
        changed
    }

    fn present_frame_buffer(&mut self, fb: FrameBuffer) {
        let image = egui::ColorImage::from_rgba_premultiplied(
            [fb.resolution.0, fb.resolution.1],
            &fb.rgba_bytes,
        );

        self.texture.set(image, TextureOptions::NEAREST);
        // If its a 1x1 resolution frame buffer then the emulator display is disabled
        self.last_resolution = (fb.resolution.0 * fb.resolution.1 > 1).then_some(fb.resolution);
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Process all the input events
        if !self.is_paused {
            let mut input_dirty = false;
            input_dirty |= self.process_gamepad_events();
            input_dirty |= self.process_keyboard_events(ctx);

            if input_dirty {
                let _ = self
                    .input_tx
                    .try_send(UiCommand::NewInputState(self.input_state.clone()));
            }
        }

        // Get framebuffers from emulator thread
        match self.frame_rx.try_recv() {
            Ok(fb) => self.present_frame_buffer(fb),
            Err(TryRecvError::Empty) => (), // Do nothing
            Err(TryRecvError::Disconnected) => {
                info!("emulator thread exited, closing UI");
                return ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        };

        // Draw UI
        show_top_menu(self, ctx);
        show_info_modal(&mut self.info_modal_open, ctx);
        show_performance_panel(self, ctx, frame);
        show_central_panel(self, ctx);
    }
}

fn show_central_panel(app: &Application, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            // No resolution means show a 4:3 black screen
            let resolution = app.last_resolution.unwrap_or((4, 3));
            ui.centered_and_justified(|ui| {
                ui.add(
                    egui::Image::from_texture(SizedTexture::new(
                        &app.texture,
                        egui::vec2(resolution.0 as f32, resolution.1 as f32),
                    ))
                    .shrink_to_fit(),
                );
            });
        });
}

fn show_top_menu(app: &mut Application, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            egui::widgets::global_theme_preference_switch(ui);
            ui.separator();

            ui.menu_button("System", |ui| {
                ui.add_enabled(false, egui::Button::new("Start File"));

                ui.add_enabled(false, egui::Button::new("Start Bios"));

                let label = if app.is_paused { "Resume" } else { "Pause" };
                if ui.button(label).clicked() {
                    app.input_tx // Blocking send, must succeed
                        .send(UiCommand::SetPaused(!app.is_paused))
                        .unwrap();
                    app.is_paused = !app.is_paused;
                }

                if ui.button("Restart").clicked() {
                    app.input_tx.send(UiCommand::Restart).unwrap();
                }

                if ui.button("Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Settings", |ui| {
                ui.add_enabled(false, egui::Button::new("BIOS Settings"));

                ui.add_enabled(false, egui::Button::new("Games Directory"));

                ui.add_enabled(false, egui::Button::new("Keybinds"));

                ui.add_enabled(false, egui::Button::new("Switch Renderer"));
            });

            ui.menu_button("Debug", |ui| {
                ui.add_enabled(false, egui::Button::new("Open Debugger View"));

                ui.add_enabled(false, egui::Button::new("Show VRAM"));
            });

            ui.menu_button("Help", |ui| {
                ui.hyperlink_to("Source Code", "https://github.com/kaezrr/starpsx");
                ui.hyperlink_to(
                    "Report an Issue",
                    "https://github.com/kaezrr/starpsx/issues/new",
                );

                ui.separator();
                if ui.button("About StarPSX").clicked() {
                    app.info_modal_open = true;
                }
            });
        });
    });
}

fn show_info_modal(show_modal: &mut bool, ctx: &egui::Context) {
    if *show_modal {
        let modal = egui::Modal::new(egui::Id::new("Info")).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("About StarPSX");
            });

            ui.separator();
            ui.monospace(format!(
                "Version: {}-{}\nPlatform: {} {}",
                env!("CARGO_PKG_VERSION"),
                option_env!("GIT_HASH").unwrap_or("unknown"),
                std::env::consts::OS,
                std::env::consts::ARCH,
            ));

            ui.separator();
            ui.label("StarPSX is a free and open source Playstation 1 emulator.");
            ui.label("It aims to be cross-platform and easy to use.");

            ui.separator();
            ui.monospace("Author: Anjishnu Banerjee <kaezr.dev@gmail.com>");

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Source:");
                ui.hyperlink_to("Github", "https://github.com/kaezrr/starpsx");
                ui.label("License: GPLv3");
            });

            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                if ui.button("Close").clicked() {
                    ui.close();
                }
            })
        });

        if modal.should_close() {
            *show_modal = false;
        }
    }
}

fn show_performance_panel(app: &Application, ctx: &egui::Context, frame: &eframe::Frame) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            let (fps, core_ms) = app.shared_metrics.load();
            ui.label(format!("FPS: {}", fps));
            ui.separator();
            ui.label(format!("Core: {:.2} ms", core_ms));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(render_state) = frame.wgpu_render_state() {
                    let info = &render_state.adapter.get_info();
                    ui.label(format!("Software Renderer ({:?})", info.backend));
                }

                ui.separator();
                ui.label(match app.last_resolution {
                    Some((w, h)) => format!("{w}x{h}"),
                    None => "No Image".into(),
                });
            })
        })
    });
}
