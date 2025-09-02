pub mod utils;

use crate::utils::Color;

const CANVAS_WIDTH: usize = 1024;
const CANVAS_HEIGHT: usize = 512;

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
    pub fn frame_buffer(&self) -> &[u32] {
        bytemuck::cast_slice(self.pixel_buffer.as_ref())
    }

    pub fn copy_vram_fb(&mut self, vram: &[u8], sx: u16, sy: u16, width: usize, height: usize) {
        let sx = sx as usize;
        let sy = sy as usize;

        for y in 0..height {
            for x in 0..width {
                let vram_addr = 2 * ((sy + y) * 1024 + (sx + x));
                let pixel = u16::from_le_bytes([vram[vram_addr], vram[vram_addr + 1]]);
                self.pixel_buffer[width * y + x] = Color::new_5bit(pixel);
            }
        }
    }
}
