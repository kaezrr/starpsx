use std::error::Error;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};
use std::time::Duration;

use eframe::egui::{self, Color32, ColorImage, vec2};
use eframe::egui::{TextureOptions, ViewportCommand, load::SizedTexture};
use egui_extras::Column;
use egui_notify::Toasts;
use futures::FutureExt;
use rfd::{AsyncFileDialog, FileHandle};
use starpsx_renderer::FrameBuffer;
use tracing::{error, info, trace};

use crate::config::{self, LaunchConfig, RunnablePath};
use crate::emulator::{self, CoreMetrics, UiCommand};
use crate::input::{self, ActionValue, PhysicalInput};

pub struct Application {
    gamepad: gilrs::Gilrs,
    input_state: input::GamepadState,

    app_config: config::AppConfig,
    config_path: PathBuf,

    app_state: Option<AppState>,
    egui_ctx: egui::Context,

    last_run: Option<RunnablePath>,

    // GUI states
    toasts: egui_notify::Toasts,

    show_keybinds: bool,
    info_modal_open: bool,
    bios_modal_open: bool,

    pending_dialog: Option<PendingDialog>,
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Err(err) = self.poll_dialog() {
            error!(%err, "error loading file");
            self.toasts.error(format!("error loading file: {err}"));
        }

        show_keybinds(&mut self.show_keybinds, ctx);

        show_top_menu(self, ctx);

        show_info_modal(&mut self.info_modal_open, ctx);

        show_bios_modal(self, ctx);

        show_performance_panel(self, ctx, frame);

        if let Some(mut emu) = self.app_state.take() {
            // Process all the input events
            if !emu.is_paused {
                let mut input_dirty = false;
                let was_analog = self.input_state.analog_mode;

                input_dirty |= self.process_gamepad_events();
                input_dirty |= self.process_keyboard_events(ctx);

                let is_analog = self.input_state.analog_mode;

                if was_analog != is_analog {
                    let msg = if is_analog {
                        "Controller switched to analog mode"
                    } else {
                        "Controller switched to digital mode"
                    };

                    self.toasts.info(msg).duration(Duration::from_secs(2));
                }

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
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Welcome to StarPSX")
                            .size(32.0)
                            .strong(),
                    );
                    ui.label("Please select a valid PS1 BIOS image in the \"BIOS Settings\" before trying to start a file!")
                });
            });
        }

        self.toasts.show(ctx);
    }
}

impl Application {
    pub fn new(cc: &eframe::CreationContext<'_>, launch_config: LaunchConfig) -> Self {
        let mut app = Self {
            egui_ctx: cc.egui_ctx.clone(),

            gamepad: gilrs::Gilrs::new().expect("could not initalize gilrs"),
            input_state: input::GamepadState::default(),

            app_state: None,
            last_run: None,

            app_config: launch_config.app_config,
            config_path: launch_config.config_path,

            toasts: Toasts::default().with_margin(vec2(5.0, 40.0)),

            show_keybinds: false,
            info_modal_open: false,
            bios_modal_open: false,

            pending_dialog: None,
        };

        if launch_config.auto_run {
            let result = match launch_config.runnable_path {
                Some(file) => app.start_file(file),
                None => app.start_bios(),
            };

            if let Err(err) = result {
                error!(%err, "could not auto start emulator");
                app.toasts
                    .error(format!("Could not auto start emulator: {err}"));
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

                gilrs::EventType::Connected => {
                    info!("gamepad connected");
                    self.toasts.info("Gamepad Connected!");
                }

                gilrs::EventType::Disconnected => {
                    info!("gamepad disconnected");
                    self.toasts.info("Gamepad Disconnected!");
                }

                _ => trace!(?event, "gamepad event ignored"),
            }
        }
        changed
    }

    fn get_metrics(&self) -> MetricsSnapshot {
        if let Some(ref emu) = self.app_state {
            let (frame_ms, core_ms) = emu.shared_metrics.load();
            MetricsSnapshot {
                fps: (1.0 / frame_ms).round() as u32,
                core_fps: (1.0 / core_ms).round() as u32,
                core_ms: core_ms * 1000.0,
                resolution: emu.last_resolution,
            }
        } else {
            MetricsSnapshot::default()
        }
    }

    fn is_paused(&self) -> bool {
        self.app_state
            .as_ref()
            .map(|a| a.is_paused)
            .unwrap_or(false)
    }

    fn vram_display_on(&self) -> bool {
        self.app_config.display_vram
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
            self.app_config.display_vram,
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

    fn stop_emulator(&mut self) {
        if let Some(state) = self.app_state.take() {
            state.shutdown();
        }
    }

    fn toggle_pause(&mut self) {
        if let Some(ref mut state) = self.app_state {
            state.toggle_pause();
            if state.is_paused {
                self.toasts.warning("Paused").duration(None).closable(false);
            } else {
                self.toasts.dismiss_all_toasts();
            }
        }
    }

    fn restart_emulator(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.app_state.take() {
            state.shutdown();
        }

        let last_run = self.last_run.take();
        self.start_emulator(last_run)?;
        Ok(())
    }

    fn start_bios(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.app_state.take() {
            state.shutdown();
        }

        self.start_emulator(None)?;
        Ok(())
    }

    fn start_file(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        if let Some(state) = self.app_state.take() {
            state.shutdown();
        }

        let runnable = emulator::parse_runnable(path)?;
        self.start_emulator(Some(runnable))?;
        Ok(())
    }

    fn toggle_vram_display(&mut self) {
        if let Some(ref mut state) = self.app_state {
            state.set_vram_display(!self.app_config.display_vram);
        }
        self.app_config.display_vram = !self.app_config.display_vram;
        self.app_config.save_to_file(&self.config_path);
    }

    fn poll_dialog(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(dialog) = self.pending_dialog.as_mut() else {
            return Ok(());
        };

        match dialog {
            PendingDialog::SelectBios(fut) => {
                if let Some(result) = fut.now_or_never() {
                    self.pending_dialog = None;
                    if let Some(file) = result {
                        self.app_config.bios_path = Some(file.path().to_path_buf());
                        self.app_config.save_to_file(&self.config_path);
                    }
                }
            }
            PendingDialog::SelectFile(fut) => {
                if let Some(result) = fut.now_or_never() {
                    self.pending_dialog = None;
                    if let Some(file) = result {
                        self.start_file(file.path().to_path_buf())?;
                    }
                }
            }
        }

        Ok(())
    }
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

    fn send_blocking_cmd(&mut self, cmd: UiCommand) {
        self.input_tx.send(cmd).unwrap();
    }

    fn toggle_pause(&mut self) {
        self.send_blocking_cmd(UiCommand::SetPaused(!self.is_paused));
        self.is_paused = !self.is_paused;
    }

    fn set_vram_display(&mut self, is_enabled: bool) {
        self.send_blocking_cmd(UiCommand::SetVramDisplay(is_enabled));
    }

    fn shutdown(mut self) {
        self.send_blocking_cmd(UiCommand::Shutdown);
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
                                    .set_title("Select file to Run")
                                    .pick_file(),
                            )));
                        }

                        if ui.button("Start BIOS").clicked() {
                            app.start_bios().unwrap_or_else(|err| {
                                error!(%err, "could not start bios");
                                app.toasts.error(format!("Could not start bios: {err}"));
                            })
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
                        app.restart_emulator().unwrap_or_else(|err| {
                            error!(%err, "could not restart emulator");
                            app.toasts
                                .error(format!("Could not restart emulator: {err}"));
                        });
                    }

                    if ui.button("Stop").clicked() {
                        app.stop_emulator();
                    }
                });

                if ui.button("Exit").clicked() {
                    app.stop_emulator();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Settings", |ui| {
                if ui.button("BIOS Settings").clicked() {
                    app.bios_modal_open = true;
                }

                if ui.button("Keybinds").clicked() {
                    app.show_keybinds = true;
                }
            });

            ui.menu_button("Debug", |ui| {
                ui.add_enabled(false, egui::Button::new("Open Debugger View"));

                let label = if app.vram_display_on() {
                    "Hide VRAM"
                } else {
                    "Show VRAM"
                };

                if ui.button(label).clicked() {
                    app.toggle_vram_display();
                }
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
            ui.label(format!("Core: {:.2} ms ({} FPS)", m.core_ms, m.core_fps));

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
        ui.set_width(400.0);
        ui.heading("Select BIOS image");
        ui.add_space(10.0);

        ui.label("Selected:");
        ui.horizontal_wrapped(|ui| match &app.app_config.bios_path {
            Some(path) => {
                ui.monospace(path.display().to_string());
            }
            None => {
                ui.colored_label(ui.visuals().error_fg_color, "No BIOS image selected");
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
                            .set_title("Select PS1 BIOS image")
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

fn show_keybinds(open: &mut bool, ctx: &egui::Context) {
    egui::Window::new("Keybinds")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .open(open)
        .show(ctx, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Action");
                    });
                    header.col(|ui| {
                        ui.strong("Controller");
                    });
                    header.col(|ui| {
                        ui.strong("Keyboard");
                    });
                })
                .body(|mut body| {
                    for keybind in config::KEYBIND_ROWS {
                        body.row(30.0, |mut row| {
                            row.col(|ui| {
                                ui.label(keybind.action);
                            });
                            row.col(|ui| {
                                ui.label(keybind.controller);
                            });
                            row.col(|ui| {
                                ui.label(keybind.keyboard);
                            });
                        });
                    }
                })
        });
}

#[derive(Default)]
struct MetricsSnapshot {
    fps: u32,
    core_fps: u32,
    core_ms: f32,
    resolution: Option<(usize, usize)>,
}

enum PendingDialog {
    SelectBios(Pin<Box<dyn Future<Output = Option<FileHandle>>>>),
    SelectFile(Pin<Box<dyn Future<Output = Option<FileHandle>>>>),
}
