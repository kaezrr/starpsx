use std::pin::Pin;

use rfd::FileHandle;

#[derive(Default)]
pub struct MetricsSnapshot {
    pub fps: u32,
    pub core_fps: u32,
    pub core_ms: f32,
    pub resolution: Option<(usize, usize)>,
}

pub enum PendingDialog {
    SelectBios(Pin<Box<dyn Future<Output = Option<FileHandle>>>>),
    SelectFile(Pin<Box<dyn Future<Output = Option<FileHandle>>>>),
}
