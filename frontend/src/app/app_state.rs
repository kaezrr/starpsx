use std::sync::mpsc::Receiver;

use eframe::egui::{self, TextureOptions};
use starpsx_renderer::FrameBuffer;

use crate::{debugger::Debugger, emulator::UiCommand};

// This holds all the state required after emulator init
pub struct AppState {
    pub debugger: Debugger,

    pub frame_rx: Receiver<FrameBuffer>,

    pub texture: egui::TextureHandle,
    /// ([width, height], was_interlaced)
    pub last_frame_state: Option<([usize; 2], bool)>,
}

impl AppState {
    pub fn present_frame_buffer(&mut self, fb: FrameBuffer) {
        let image = egui::ColorImage::from_rgba_unmultiplied(fb.resolution, &fb.rgba_bytes);
        self.texture.set(image, TextureOptions::NEAREST);

        // If its a 1x1 resolution frame buffer then the emulator display is disabled
        if fb.resolution[0] * fb.resolution[1] <= 1 {
            self.last_frame_state = None;
            return;
        }

        // Non interlaced displays have their rows duplicated so divide by 2
        self.last_frame_state = if fb.is_interlaced {
            Some((fb.resolution, true))
        } else {
            Some(([fb.resolution[0], fb.resolution[1] / 2], false))
        };
    }

    pub fn set_vram_display(&mut self, is_enabled: bool) {
        self.debugger
            .sync_send(UiCommand::SetVramDisplay(is_enabled));
    }

    pub fn shutdown(self) {
        self.debugger.sync_send(UiCommand::Shutdown);
    }

    pub fn restart(&self) {
        self.debugger.sync_send(UiCommand::Restart);
    }
}
