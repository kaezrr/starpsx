use std::error::Error;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};

use eframe::egui::{self, Color32, ColorImage};
use eframe::egui::{TextureOptions, ViewportCommand, load::SizedTexture};
use futures::FutureExt;
use rfd::{AsyncFileDialog, FileHandle};
use starpsx_renderer::FrameBuffer;
use tracing::{error, info, trace};

use crate::config::{self, LaunchConfig, RunnablePath};
use crate::emulator::{self, CoreMetrics, UiCommand};
use crate::input::{self, ActionValue, PhysicalInput};

enum PendingDialog {
    SelectBios(Pin<Box<dyn Future<Output = Option<FileHandle>>>>),
    SelectFile(Pin<Box<dyn Future<Output = Option<FileHandle>>>>),
}

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

    last_run: Option<RunnablePath>,

    // GUI states
    info_modal_open: bool,
    bios_modal_open: bool,

    pending_dialog: Option<PendingDialog>,
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
            bios_path,
            &runnable_path,
        )?;

        self.last_run = runnable_path;

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
        let mut app = Self {
            egui_ctx: cc.egui_ctx.clone(),

            gamepad: gilrs::Gilrs::new().expect("could not initalize gilrs"),
            input_state: input::GamepadState::default(),

            app_state: None,
            last_run: None,

            app_config: launch_config.app_config,
            config_path: launch_config.config_path,

            info_modal_open: false,
            bios_modal_open: false,
            pending_dialog: None,
        };

        if launch_config.auto_run {
            match launch_config.runnable_path {
                Some(file) => app.load_file(file).unwrap_or_else(|err| {
                    error!(%err, "error loading file");
                }),
                None => app.start_bios(),
            }
        }

        app
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

    fn is_paused(&self) -> bool {
        self.app_state
            .as_ref()
            .map(|a| a.is_paused)
            .unwrap_or(false)
    }

    fn quit_game(&mut self) {
        if let Some(state) = self.app_state.take() {
            state.shutdown();
        }
    }

    fn toggle_pause(&mut self) {
        if let Some(ref mut state) = self.app_state {
            state.toggle_pause();
        }
    }

    fn restart(&mut self) {
        if let Some(state) = self.app_state.take() {
            state.shutdown();
        }

        let last_run = self.last_run.take();
        let _ = self.start_emulator(last_run);
    }

    fn start_bios(&mut self) {
        if let Some(state) = self.app_state.take() {
            state.shutdown();
        }

        let _ = self.start_emulator(None);
    }

    fn load_file(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.app_state.take() {
            state.shutdown();
        }

        let runnable = emulator::parse_runnable(path)?;
        let _ = self.start_emulator(Some(runnable));
        Ok(())
    }

    fn poll_dialog(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(dialog) = self.pending_dialog.as_mut() else {
            return Ok(());
        };

        match dialog {
            PendingDialog::SelectBios(fut) => {
                if let Some(result) = fut.now_or_never() {
                    if let Some(file) = result {
                        self.app_config.bios_path = Some(file.path().to_path_buf());
                        self.app_config.save_to_file(&self.config_path);
                    }
                    self.pending_dialog = None;
                }
            }
            PendingDialog::SelectFile(fut) => {
                if let Some(result) = fut.now_or_never() {
                    if let Some(file) = result {
                        self.load_file(file.path().to_path_buf())?;
                    }
                    self.pending_dialog = None;
                }
            }
        }

        Ok(())
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

    fn shutdown(&self) {
        self.input_tx.send(UiCommand::Shutdown).unwrap();
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Err(err) = self.poll_dialog() {
            error!(%err, "error loading file");
        }

        show_top_menu(self, ctx);

        show_info_modal(&mut self.info_modal_open, ctx);
        show_bios_modal(self, ctx);

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
        } else {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Welcome to StarPSX")
                            .size(32.0)
                            .strong(),
                    );
                });
            });
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
                // Only if a valid bios is set and emulator is not running
                ui.add_enabled_ui(
                    app.app_config.bios_path.is_some() && app.app_state.is_none(),
                    |ui| {
                        if ui.button("Start File").clicked() {
                            app.pending_dialog = Some(PendingDialog::SelectFile(Box::pin(
                                AsyncFileDialog::new()
                                    .add_filter("Game", &["bin", "BIN", "cue", "exe"])
                                    .set_title("Select File to Run")
                                    .pick_file(),
                            )));
                        }

                        if ui.button("Start BIOS").clicked() {
                            app.start_bios();
                        }
                    },
                );

                // Only if emulator is running
                ui.add_enabled_ui(app.app_state.is_some(), |ui| {
                    let label = if app.is_paused() { "Resume" } else { "Pause" };
                    if ui.button(label).clicked() {
                        app.toggle_pause();
                    }

                    if ui.button("Restart").clicked() {
                        app.restart();
                    }

                    if ui.button("Stop").clicked() {
                        app.quit_game();
                    }
                });

                if ui.button("Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Settings", |ui| {
                if ui.button("BIOS Settings").clicked() {
                    app.bios_modal_open = true;
                }

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

fn show_bios_modal(app: &mut Application, ctx: &egui::Context) {
    if !app.bios_modal_open {
        return;
    }

    let modal = egui::Modal::new(egui::Id::new("Info")).show(ctx, |ui| {
        ui.set_min_width(200.0);

        ui.heading("Select BIOS");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Current BIOS:");
            match &app.app_config.bios_path {
                Some(path) => {
                    ui.monospace(path.display().to_string());
                }
                None => {
                    ui.colored_label(ui.visuals().error_fg_color, "No BIOS selected");
                }
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        egui::containers::Sides::new().show(
            ui,
            |ui| {
                if ui.button("Choose BIOS File…").clicked() {
                    app.pending_dialog = Some(PendingDialog::SelectBios(Box::pin(
                        AsyncFileDialog::new()
                            .add_filter("PlayStation BIOS", &["bin", "BIN"])
                            .set_title("Select PS1 BIOS Image")
                            .pick_file(),
                    )));
                }
            },
            |ui| {
                if ui.button("Close").clicked() {
                    ui.close();
                }
            },
        );
    });

    if modal.should_close() {
        app.bios_modal_open = false;
    }
}
