use crate::utils::Color;
pub mod utils;

pub const CANVAS_WIDTH: usize = 1024;
pub const CANVAS_HEIGHT: usize = 512;

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

    pub fn copy_vram(&mut self, vram: &[u8]) {
        for y in 0..512 {
            for x in 0..1024 {
                let vram_addr = 2 * (y * 1024 + x);
                let pixel = u16::from_le_bytes([vram[vram_addr], vram[vram_addr + 1]]);
                self.pixel_buffer[1024 * y + x] = Color::new_5bit(pixel);
            }
        }
    }

    pub fn copy_vram_fb(&mut self, vram: &[u8], start_x: u16, start_y: u16) {
        let start_x = start_x as usize;
        let start_y = start_y as usize;

        for y in start_y..(start_y + 240) {
            for x in start_x..(start_x + 320) {
                let vram_addr = 2 * (y * 1024 + x);
                let pixel = u16::from_le_bytes([vram[vram_addr], vram[vram_addr + 1]]);
                self.pixel_buffer[320 * y + x] = Color::new_5bit(pixel);
            }
        }
    }
}
