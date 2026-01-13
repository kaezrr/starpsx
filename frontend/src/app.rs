use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};

use cpal::traits::StreamTrait;
use eframe::egui::{self, Color32, ColorImage};
use eframe::egui::{TextureOptions, ViewportCommand, load::SizedTexture};
use starpsx_renderer::FrameBuffer;
use tracing::{info, trace};

use crate::input::{self, ActionValue, GamepadState, PhysicalInput};

pub struct Application {
    gamepad: gilrs::Gilrs,
    input_state: input::GamepadState,
    keybindings: input::Bindings,

    frame_rx: Receiver<FrameBuffer>,
    input_tx: SyncSender<GamepadState>,

    texture: egui::TextureHandle,

    audio_stream: cpal::Stream,
    is_paused: bool,

    fps: f64,
    last_frame_time: std::time::Instant,
    last_resolution: Option<(usize, usize)>,
}

impl Application {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        frame_rx: Receiver<FrameBuffer>,
        input_tx: SyncSender<GamepadState>,
        audio_stream: cpal::Stream,
    ) -> Self {
        // Start playing the audio
        audio_stream.play().unwrap();

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

            audio_stream,
            is_paused: false,

            // Performance metrics
            fps: 0.0,
            last_frame_time: std::time::Instant::now(),
            last_resolution: None,
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
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Process all the input events
        if !self.is_paused {
            let mut input_dirty = false;
            input_dirty |= self.process_gamepad_events();
            input_dirty |= self.process_keyboard_events(ctx);

            if input_dirty {
                let _ = self.input_tx.try_send(self.input_state.clone());
            }
        }

        // Get framebuffers from emulator thread
        match self.frame_rx.try_recv() {
            Ok(FrameBuffer {
                rgba_bytes,
                resolution,
            }) => {
                let now = std::time::Instant::now();
                let dt = now.duration_since(self.last_frame_time).as_secs_f64();
                self.last_frame_time = now;
                self.fps = 1.0 / dt;

                self.last_resolution = (resolution.0 * resolution.1 > 1).then_some(resolution);

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
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                egui::widgets::global_theme_preference_switch(ui);
                ui.separator();

                ui.menu_button("System", |ui| {
                    ui.add_enabled(false, egui::Button::new("Start File"));

                    ui.add_enabled(false, egui::Button::new("Start Bios"));

                    let label = if self.is_paused { "Resume" } else { "Pause" };
                    if ui.button(label).clicked() {
                        self.is_paused = !self.is_paused;
                        match self.is_paused {
                            true => self.audio_stream.pause().unwrap(),
                            false => self.audio_stream.play().unwrap(),
                        }
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.add_enabled(false, egui::Button::new("Settings"));

                ui.add_enabled(false, egui::Button::new("Debug"));

                use egui::special_emojis::GITHUB;
                ui.menu_button("Help", |ui| {
                    ui.hyperlink_to(
                        format!("{GITHUB} Source Code"),
                        "https://github.com/kaezrr/starpsx",
                    );
                    ui.hyperlink_to(
                        "Report an issue",
                        "https://github.com/kaezrr/starpsx/issues/new",
                    )
                });
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("FPS: {:.0}", self.fps));
                ui.separator();
                ui.label(format!("Core: {:.2} ms", 12));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(render_state) = frame.wgpu_render_state() {
                        let info = &render_state.adapter.get_info();
                        ui.label(format!("Software Renderer ({:?})", info.backend));
                    }

                    ui.separator();
                    ui.label(match self.last_resolution {
                        Some((w, h)) => format!("{w}x{h}"),
                        None => "No Image".into(),
                    });
                })
            })
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let size = ui.available_size();
                ui.image(SizedTexture::new(&self.texture, size));
            });
    }
}
