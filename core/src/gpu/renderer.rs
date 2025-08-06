pub const CANVAS_WIDTH: usize = 640;
pub const CANVAS_HEIGHT: usize = 480;

pub struct Renderer {
    pub pixel_buffer: Vec<u8>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            pixel_buffer: vec![0; CANVAS_HEIGHT * CANVAS_WIDTH * 3],
        }
    }
}
