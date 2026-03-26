use std::pin::Pin;
use std::time::Duration;
use std::time::Instant;

use rfd::FileHandle;

pub struct MetricsSnapshot {
    pub fps: f32,
    pub frame_counter: u32,
    pub last_frame_refresh: Instant,
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
    pub fn capture_frame(&mut self) {
        self.frame_counter += 1;

        let elapsed = self.last_frame_refresh.elapsed();
        if elapsed >= Duration::from_millis(500) {
            let seconds = elapsed.as_secs_f32();
            self.fps = self.frame_counter as f32 / seconds;
            self.frame_counter = 0;
            self.last_frame_refresh = Instant::now();
        }
    }
}
pub enum PendingDialog {
    SelectBios(Pin<Box<dyn Future<Output = Option<FileHandle>>>>),
    SelectFile(Pin<Box<dyn Future<Output = Option<FileHandle>>>>),
}
