#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::mpsc::{Receiver, TryRecvError};

use eframe::egui::{self, ColorImage, TextureOptions, ViewportCommand, load::SizedTexture};
use starpsx_renderer::FrameBuffer;
use tracing::info;

pub struct Application {
    // gamepad: gilrs::Gilrs,
    texture: egui::TextureHandle,
    frame_receiver: Receiver<FrameBuffer>,
}

impl Application {
    pub fn new(cc: &eframe::CreationContext<'_>, frame_receiver: Receiver<FrameBuffer>) -> Self {
        Self {
            // gamepad: gilrs::Gilrs::new().expect("could not initalize gilrs"),
            frame_receiver,
            texture: cc.egui_ctx.load_texture(
                "frame buffer",
                ColorImage::example(),
                egui::TextureOptions::NEAREST,
            ),
        }
    }

    // fn process_gamepad_events(&mut self) {
    //     let psx_gamepad = self.emulator.gamepad_mut();
    //
    //     while let Some(gilrs::Event { event, .. }) = self.gamepad.next_event() {
    //         match event {
    //             gilrs::EventType::ButtonPressed(gilrs::Button::Mode, _) => {
    //                 // Eat this event
    //             }
    //
    //             gilrs::EventType::ButtonReleased(gilrs::Button::Mode, _) => {
    //                 psx_gamepad.toggle_analog_mode()
    //             }
    //
    //             gilrs::EventType::ButtonPressed(button, _) => {
    //                 psx_gamepad.set_button_state(utils::convert_button(button), true)
    //             }
    //
    //             gilrs::EventType::ButtonReleased(button, _) => {
    //                 psx_gamepad.set_button_state(utils::convert_button(button), false)
    //             }
    //
    //             gilrs::EventType::Connected => {
    //                 info!("gamepad connected")
    //             }
    //
    //             gilrs::EventType::Disconnected => {
    //                 info!("gamepad disconnected")
    //             }
    //
    //             gilrs::EventType::AxisChanged(axis, value, _) => {
    //                 let (converted_axis, new_value) = utils::convert_axis(axis, value);
    //                 psx_gamepad.set_stick_axis(converted_axis, new_value);
    //             }
    //
    //             _ => trace!("gamepad event ignored: {event:?}"),
    //         }
    //     }
    // }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        match self.frame_receiver.try_recv() {
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
