use std::sync::{Arc, mpsc::Receiver};

use eframe::egui::{self, TextureOptions};
use starpsx_renderer::FrameBuffer;

use crate::{
    debugger::Debugger,
    emulator::{CoreMetrics, UiCommand},
};

// This holds all the state required after emulator init
pub struct AppState {
    pub debugger: Debugger,

    pub frame_rx: Receiver<FrameBuffer>,
    pub shared_metrics: Arc<CoreMetrics>,

    pub texture: egui::TextureHandle,
    pub last_resolution: Option<(usize, usize)>,
}

impl AppState {
    pub fn present_frame_buffer(&mut self, fb: FrameBuffer) {
        let image = egui::ColorImage::from_rgba_premultiplied(
            [fb.resolution.0, fb.resolution.1],
            &fb.rgba_bytes,
        );

        self.texture.set(image, TextureOptions::NEAREST);

        // If its a 1x1 resolution frame buffer then the emulator display is disabled
        self.last_resolution = (fb.resolution.0 * fb.resolution.1 > 1).then_some(fb.resolution);
    }

    pub fn set_vram_display(&mut self, is_enabled: bool) {
        self.debugger
            .sync_send(UiCommand::SetVramDisplay(is_enabled));
    }

    pub fn shutdown(mut self) {
        self.debugger.sync_send(UiCommand::Shutdown);
    }
}
