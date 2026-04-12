use std::pin::Pin;
use std::time::Duration;
use std::time::Instant;

use rfd::FileHandle;
use starpsx_renderer::FrameBuffer;

pub struct MetricsSnapshot {
    pub fps: f32,
    pub frame_counter: u32,
    pub last_frame_refresh: Instant,

    /// (width, height, interlaced)
    pub last_frame_data: Option<([usize; 2], bool)>,
}

impl Default for MetricsSnapshot {
    fn default() -> Self {
        Self {
            fps: 0.,
            frame_counter: 0,
            last_frame_refresh: Instant::now(),
            last_frame_data: None,
        }
    }
}

impl MetricsSnapshot {
    pub fn capture_frame_data(&mut self, fb: &FrameBuffer) {
        self.frame_counter += 1;

        let elapsed = self.last_frame_refresh.elapsed();
        if elapsed >= Duration::from_millis(500) {
            let seconds = elapsed.as_secs_f32();
            self.fps = self.frame_counter as f32 / seconds;
            self.frame_counter = 0;
            self.last_frame_refresh = Instant::now();
        }

        // If its a 1x1 resolution frame buffer then the emulator display is disabled
        if fb.resolution[0] * fb.resolution[1] <= 1 {
            self.last_frame_data = None;
            return;
        }

        // Non interlaced displays have their rows duplicated so divide by 2
        self.last_frame_data = if fb.is_interlaced {
            Some((fb.resolution, true))
        } else {
            Some(([fb.resolution[0], fb.resolution[1] / 2], false))
        };
    }
}
pub enum PendingDialog {
    SelectBios(Pin<Box<dyn Future<Output = Option<FileHandle>>>>),
    SelectFile(Pin<Box<dyn Future<Output = Option<FileHandle>>>>),
}
