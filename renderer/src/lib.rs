use crate::utils::Color;
pub mod utils;

pub const CANVAS_WIDTH: usize = 1024;
pub const CANVAS_HEIGHT: usize = 512;

const CANVAS_BOUND_X: i16 = (CANVAS_WIDTH / 2) as i16;
const CANVAS_BOUND_Y: i16 = (CANVAS_HEIGHT / 2) as i16;

pub struct Renderer {
    pub pixel_buffer: Box<[Color; CANVAS_HEIGHT * CANVAS_WIDTH]>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            pixel_buffer: Box::new([Color::default(); CANVAS_HEIGHT * CANVAS_WIDTH]),
        }
    }
}

impl Renderer {
    pub fn put_pixel(&mut self, x: i16, y: i16, color: Color) {
        let x = x + CANVAS_BOUND_X;
        let y = CANVAS_BOUND_Y - y;

        let idx = (y as usize) * CANVAS_WIDTH + (x as usize);
        self.pixel_buffer[idx] = color;
    }

    pub fn frame_buffer(&self) -> &[u32] {
        bytemuck::cast_slice(self.pixel_buffer.as_ref())
    }

    pub fn copy_vram(&mut self, vram: &[u8]) {
        for y in 0..512 {
            for x in 0..1024 {
                let vram_addr = 2 * (y * 1024 + x);
                let pixel = u16::from_le_bytes([vram[vram_addr], vram[vram_addr + 1]]);
                self.pixel_buffer[1024 * y + x] = Color::new(pixel);
            }
        }
    }
}
