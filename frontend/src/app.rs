use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};

use eframe::egui::{self, Color32, ColorImage};
use eframe::egui::{TextureOptions, ViewportCommand, load::SizedTexture};
use starpsx_renderer::FrameBuffer;
use tracing::{error, info, trace};

use crate::config::{self, LaunchConfig, RunnablePath};
use crate::emulator::{self, CoreMetrics, UiCommand};
use crate::input::{self, ActionValue, PhysicalInput};

// This holds all the state required after emulator init
struct AppState {
    frame_rx: Receiver<FrameBuffer>,
    input_tx: SyncSender<UiCommand>,

    texture: egui::TextureHandle,

    is_paused: bool,

    // metrics
    shared_metrics: Arc<CoreMetrics>,
    last_resolution: Option<(usize, usize)>,
}

pub struct Application {
    gamepad: gilrs::Gilrs,
    input_state: input::GamepadState,

    app_config: config::AppConfig,
    config_path: PathBuf,

    app_state: Option<AppState>,
    egui_ctx: egui::Context,

    // GUI states
    info_modal_open: bool,
}

#[derive(Default)]
struct MetricsSnapshot {
    fps: u32,
    core_ms: f32,
    resolution: Option<(usize, usize)>,
}

impl Application {
    fn get_metrics(&self) -> MetricsSnapshot {
        if let Some(ref emu) = self.app_state {
            let (fps, core_ms) = emu.shared_metrics.load();
            MetricsSnapshot {
                fps,
                core_ms,
                resolution: emu.last_resolution,
            }
        } else {
            MetricsSnapshot::default()
        }
    }

    fn start_emulator(
        &mut self,
        runnable_path: Option<RunnablePath>,
    ) -> Result<(), Box<dyn Error>> {
        let bios_path = self
            .app_config
            .bios_path
            .as_ref()
            .ok_or("bios path missing")?;

        // Message channels for thread communication
        let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel::<FrameBuffer>(1);
        let (input_tx, input_rx) = std::sync::mpsc::sync_channel::<UiCommand>(1);

        let shared_metrics = Arc::new(CoreMetrics::default());

        // Build emulator from the provided configuration
        let emulator = emulator::Emulator::build(
            self.egui_ctx.clone(),
            frame_tx,
            input_rx,
            shared_metrics.clone(),
            bios_path.clone(),
            runnable_path,
        )?;

        self.app_state = Some(AppState {
            frame_rx,
            input_tx,

            texture: self.egui_ctx.load_texture(
                "frame buffer",
                ColorImage::filled([100, 100], Color32::BLACK),
                egui::TextureOptions::NEAREST,
            ),

            is_paused: false,

            shared_metrics,
            last_resolution: None,
        });

        emulator.run();
        Ok(())
    }

    pub fn new(cc: &eframe::CreationContext<'_>, launch_config: LaunchConfig) -> Self {
        Self {
            egui_ctx: cc.egui_ctx.clone(),

            gamepad: gilrs::Gilrs::new().expect("could not initalize gilrs"),
            input_state: input::GamepadState::default(),

            app_state: None,

            app_config: launch_config.app_config,
            config_path: launch_config.config_path,

            info_modal_open: false,
        }
    }

    fn process_keyboard_events(&mut self, ctx: &egui::Context) -> bool {
        if ctx.wants_keyboard_input() {
            return false;
        }

        let mut changed = false;
        ctx.input(|i| {
            for (phys, action) in &self.app_config.keybinds {
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
                    if let Some(action) = self.app_config.keybinds.get(&phys) {
                        changed |= self
                            .input_state
                            .handle_action(action, ActionValue::Digital(true));
                    }
                }

                gilrs::EventType::ButtonReleased(button, _) => {
                    let phys = PhysicalInput::GilrsButton(button);
                    if let Some(action) = self.app_config.keybinds.get(&phys) {
                        changed |= self
                            .input_state
                            .handle_action(action, ActionValue::Digital(false));
                    }
                }

                gilrs::EventType::AxisChanged(axis, value, _) => {
                    let phys = PhysicalInput::GilrsAxis(axis);
                    if let Some(action) = self.app_config.keybinds.get(&phys) {
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
}

impl AppState {
    fn present_frame_buffer(&mut self, fb: FrameBuffer) {
        let image = egui::ColorImage::from_rgba_premultiplied(
            [fb.resolution.0, fb.resolution.1],
            &fb.rgba_bytes,
        );

        self.texture.set(image, TextureOptions::NEAREST);

        // If its a 1x1 resolution frame buffer then the emulator display is disabled
        self.last_resolution = (fb.resolution.0 * fb.resolution.1 > 1).then_some(fb.resolution);
    }

    fn toggle_pause(&mut self) {
        self.input_tx // Blocking send, must succeed
            .send(UiCommand::SetPaused(!self.is_paused))
            .unwrap();
        self.is_paused = !self.is_paused;
    }

    fn restart(&self) {
        self.input_tx.send(UiCommand::Restart).unwrap();
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        show_top_menu(self, ctx);

        show_info_modal(&mut self.info_modal_open, ctx);

        show_performance_panel(self, ctx, frame);

        if let Some(mut emu) = self.app_state.take() {
            // Process all the input events
            if !emu.is_paused {
                let mut input_dirty = false;
                input_dirty |= self.process_gamepad_events();
                input_dirty |= self.process_keyboard_events(ctx);

                if input_dirty {
                    let _ = emu
                        .input_tx
                        .try_send(UiCommand::NewInputState(self.input_state.clone()));
                }
            }

            // Get framebuffers from emulator thread
            match emu.frame_rx.try_recv() {
                Ok(fb) => {
                    emu.present_frame_buffer(fb);
                }
                Err(TryRecvError::Disconnected) => {
                    info!("emulator thread exited, closing UI");
                    return ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                Err(TryRecvError::Empty) => (), // Do nothing
            };

            show_central_panel(&emu, ctx);

            self.app_state = Some(emu);
        }
    }
}

fn show_central_panel(app: &AppState, ctx: &egui::Context) {
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

                if let Some(ref mut emu) = app.app_state {
                    let label = if emu.is_paused { "Resume" } else { "Pause" };
                    if ui.button(label).clicked() {
                        emu.toggle_pause();
                    }
                    if ui.button("Restart").clicked() {
                        emu.restart();
                    }
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
    if !*show_modal {
        return;
    }
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

fn show_performance_panel(app: &Application, ctx: &egui::Context, frame: &eframe::Frame) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        let m = app.get_metrics();
        ui.horizontal(|ui| {
            ui.label(format!("FPS: {}", m.fps));
            ui.separator();
            ui.label(format!("Core: {:.2} ms", m.core_ms));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(render_state) = frame.wgpu_render_state() {
                    let info = &render_state.adapter.get_info();
                    ui.label(format!("Software Renderer ({:?})", info.backend));
                }

                ui.separator();
                ui.label(match m.resolution {
                    Some((w, h)) => format!("{w}x{h}"),
                    None => "Display Off".into(),
                });
            })
        })
    });
}
