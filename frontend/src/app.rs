#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::mpsc::{Receiver, TryRecvError};

use eframe::egui::{self, Color32, ColorImage};
use eframe::egui::{TextureOptions, ViewportCommand, load::SizedTexture};
use starpsx_renderer::FrameBuffer;
use tracing::{info, trace};

use crate::util::{self, ActionValue, PhysicalInput};

pub struct Application {
    gamepad: gilrs::Gilrs,
    frame_rx: Receiver<FrameBuffer>,
    input_state: util::GamepadState,
    texture: egui::TextureHandle,
    keybindings: util::Bindings,
}

impl Application {
    pub fn new(cc: &eframe::CreationContext<'_>, frame_rx: Receiver<FrameBuffer>) -> Self {
        Self {
            gamepad: gilrs::Gilrs::new().expect("could not initalize gilrs"),
            frame_rx,
            input_state: util::GamepadState::default(),
            keybindings: util::get_default_keybinds(),
            texture: cc.egui_ctx.load_texture(
                "frame buffer",
                ColorImage::filled([100, 100], Color32::BLACK),
                egui::TextureOptions::NEAREST,
            ),
        }
    }

    fn process_keyboard_events(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }

        ctx.input(|i| {
            for (phys, action) in &self.keybindings {
                let PhysicalInput::Key(key) = phys else {
                    continue;
                };

                if i.key_pressed(*key) {
                    self.input_state
                        .handle_action(action, ActionValue::Digital(true));
                }

                if i.key_released(*key) {
                    self.input_state
                        .handle_action(action, ActionValue::Digital(false));
                }
            }
        });
    }

    fn process_gamepad_events(&mut self) {
        while let Some(gilrs::Event { event, .. }) = self.gamepad.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(button, _) => {
                    let phys = PhysicalInput::GilrsButton(button);
                    if let Some(action) = self.keybindings.get(&phys) {
                        self.input_state
                            .handle_action(action, ActionValue::Digital(true));
                    }
                }

                gilrs::EventType::ButtonReleased(button, _) => {
                    let phys = PhysicalInput::GilrsButton(button);
                    if let Some(action) = self.keybindings.get(&phys) {
                        self.input_state
                            .handle_action(action, ActionValue::Digital(false));
                    }
                }

                gilrs::EventType::AxisChanged(axis, value, _) => {
                    let phys = PhysicalInput::GilrsAxis(axis);
                    if let Some(action) = self.keybindings.get(&phys) {
                        self.input_state
                            .handle_action(action, ActionValue::Analog(value));
                    }
                }

                gilrs::EventType::Connected => info!("gamepad connected"),
                gilrs::EventType::Disconnected => info!("gamepad disconnected"),

                _ => trace!("gamepad event ignored: {event:?}"),
            }
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // Process all the input events
        self.process_gamepad_events();
        self.process_keyboard_events(ctx);

        // Get framebuffers from emulator thread
        match self.frame_rx.try_recv() {
            Ok(FrameBuffer {
                rgba_bytes,
                resolution,
            }) => {
                let image = egui::ColorImage::from_rgba_premultiplied(
                    [resolution.0, resolution.1],
                    &rgba_bytes,
                );

                self.texture.set(image, TextureOptions::NEAREST);
            }
            Err(TryRecvError::Empty) => {} // Do nothing
            Err(TryRecvError::Disconnected) => {
                info!("emulator thread exited, closing UI");
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }

        // Draw UI
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| top_menu_bar(ctx, ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let size = ui.available_size();
                ui.image(SizedTexture::new(&self.texture, size));
            });
    }
}

fn top_menu_bar(ctx: &egui::Context, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        egui::widgets::global_theme_preference_switch(ui);
        ui.menu_button("File", |ui| {
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button("Settings", |_ui| {});
        ui.menu_button("Debug", |_ui| {});
        ui.menu_button("Help", |_ui| {});
    });
}
