mod app_state;
mod ui;
mod util;

use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, mpsc::TryRecvError};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use eframe::egui::ViewportCommand;
use eframe::egui::{self, Color32, ColorImage, vec2};
use egui_notify::Toasts;
use starpsx_renderer::FrameBuffer;
use tracing::{error, info, trace};

use crate::app::app_state::AppState;
use crate::app::util::{MetricsSnapshot, PendingDialog};
use crate::config::{self, LaunchConfig, RunnablePath};
use crate::debugger::Debugger;
use crate::debugger::snapshot::DebugSnapshot;
use crate::emulator::{self, SharedState, UiChannels, UiCommand};
use crate::input::{self, ActionValue, PhysicalInput};

pub struct Application {
    gamepad: gilrs::Gilrs,
    input_state: input::GamepadState,

    app_config: config::AppConfig,
    config_path: PathBuf,

    app_state: Option<AppState>,
    egui_ctx: egui::Context,

    // GUI states
    toasts: egui_notify::Toasts,

    keybinds_table_open: bool,
    info_modal_open: bool,
    bios_modal_open: bool,

    previous_pause: bool,

    pending_dialog: Option<PendingDialog>,
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let is_paused_now = self.is_paused();

        if self.previous_pause != is_paused_now {
            self.previous_pause = is_paused_now;

            if self.previous_pause {
                self.toasts.warning("Paused").duration(None).closable(false);
            } else {
                self.toasts.dismiss_all_toasts()
            };
        }

        if let Err(err) = self.poll_dialog() {
            error!(%err, "error loading file");
            self.toasts.error(format!("error loading file: {err}"));
        }

        ui::show_keybinds(&mut self.keybinds_table_open, ctx);

        ui::show_top_menu(self, ctx);

        ui::show_info_modal(&mut self.info_modal_open, ctx);

        ui::show_bios_modal(self, ctx);

        ui::show_performance_panel(self, ctx);

        if let Some(mut emu) = self.app_state.take() {
            // Process all the input events
            if !is_paused_now {
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
                    emu.debugger
                        .send(UiCommand::NewInputState(self.input_state.clone()));
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

            if self.app_config.debugger_view {
                emu.debugger.show_ui(ctx);
            }

            ui::show_central_panel(&emu, ctx);
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

            app_config: launch_config.app_config,
            config_path: launch_config.config_path,

            toasts: Toasts::default().with_margin(vec2(5.0, 40.0)),

            keybinds_table_open: false,
            info_modal_open: false,
            bios_modal_open: false,

            previous_pause: false,

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
            let (frame_ms, core_ms) = emu.debugger.load_metrics();
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
            .map(|a| a.debugger.is_paused())
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
        let (input_tx, input_rx) = std::sync::mpsc::sync_channel::<UiCommand>(2);
        let (snapshot_tx, snapshot_rx) = std::sync::mpsc::sync_channel::<DebugSnapshot>(1);

        let shared_state = Arc::new(SharedState::default());

        // Build emulator from the provided configuration
        let emulator = emulator::Emulator::build(
            self.egui_ctx.clone(),
            UiChannels {
                frame_tx,
                input_rx,
                snapshot_tx,
            },
            shared_state.clone(),
            bios_path.clone(),
            runnable_path,
            self.app_config.display_vram,
        )?;

        self.app_state = Some(AppState {
            debugger: Debugger::new(shared_state, input_tx, snapshot_rx),

            frame_rx,
            texture: self.egui_ctx.load_texture(
                "frame buffer",
                ColorImage::filled([100, 100], Color32::BLACK),
                egui::TextureOptions::NEAREST,
            ),

            last_resolution: None,
        });

        emulator.run();
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

    fn toggle_debugger_view(&mut self) {
        self.app_config.debugger_view = !self.app_config.debugger_view;
        self.app_config.save_to_file(&self.config_path);
    }

    fn poll_dialog(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(dialog) = self.pending_dialog.as_mut() else {
            return Ok(());
        };

        let mut ctx = Context::from_waker(Waker::noop());

        match dialog {
            PendingDialog::SelectBios(fut) => {
                if let Poll::Ready(result) = fut.as_mut().poll(&mut ctx) {
                    self.pending_dialog = None;
                    if let Some(file) = result {
                        self.app_config.bios_path = Some(file.path().to_path_buf());
                        self.app_config.save_to_file(&self.config_path);
                    }
                }
            }
            PendingDialog::SelectFile(fut) => {
                if let Poll::Ready(result) = fut.as_mut().poll(&mut ctx) {
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
