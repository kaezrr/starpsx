use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;

use eframe::egui::TextureOptions;
use eframe::egui::{self};
use starpsx_renderer::FrameBuffer;

use crate::debugger::Debugger;
use crate::emulator::UiCommand;
use crate::input::GamepadState;

// This holds all the state required after emulator init
pub struct AppState {
    pub debugger: Debugger,
    pub frame_rx: Receiver<FrameBuffer>,
    pub input_tx: SyncSender<GamepadState>,
    pub texture: egui::TextureHandle,
}

impl AppState {
    pub fn present_frame_buffer(&mut self, fb: &FrameBuffer) {
        let rgba_bytes = bytemuck::cast_slice(&fb.rgba);
        let image = egui::ColorImage::from_rgba_unmultiplied(fb.resolution, rgba_bytes);

        self.texture.set(image, TextureOptions::NEAREST);
    }

    pub fn set_vram_display(&self, is_enabled: bool) {
        self.debugger
            .sync_send(UiCommand::SetVramDisplay(is_enabled));
    }

    pub fn shutdown(self) {
        self.debugger.sync_send(UiCommand::Shutdown);
    }

    pub fn set_speed(&self, val: bool) {
        self.debugger.sync_send(UiCommand::SetSpeed(val));
    }
}
