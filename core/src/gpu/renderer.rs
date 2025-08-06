pub const CANVAS_WIDTH: usize = 640;
pub const CANVAS_HEIGHT: usize = 480;

bitfield::bitfield! {
    pub struct Color(u32);
    u8, red, _ : 23, 16;
    u8, green, _ : 15, 8;
    u8, blue, _ : 7, 0;
}

impl Color {
    pub fn new(raw: u32) -> Self {
        Self(raw)
    }

    pub fn to_le_bytes(&self) -> [u8; 3] {
        [self.red(), self.blue(), self.green()]
    }
}

pub struct Renderer {
    pub pixel_buffer: Vec<u8>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            pixel_buffer: vec![0x33; CANVAS_HEIGHT * CANVAS_WIDTH * 3],
        }
    }
}

impl Renderer {
    pub fn put_pixel(&mut self, x: usize, y: usize, color: Color) {
        let color_index = y * CANVAS_WIDTH + x;

        *self.pixel_buffer[color_index..].first_chunk_mut().unwrap() = color.to_le_bytes()
    }
}
