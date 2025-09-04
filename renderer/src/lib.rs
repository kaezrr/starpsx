pub mod utils;
pub mod vec2;

use crate::{
    utils::Color,
    vec2::{Vec2, point_in_triangle},
};

const CANVAS_WIDTH: usize = 1024;
const CANVAS_HEIGHT: usize = 512;

pub struct Renderer {
    pub pixel_buffer: Box<[Color; CANVAS_HEIGHT * CANVAS_WIDTH]>,
    pub vram: Box<[u8; 512 * 2048]>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            vram: Box::new([0; 1024 * 1024]),
            pixel_buffer: Box::new([Color::default(); CANVAS_HEIGHT * CANVAS_WIDTH]),
        }
    }
}

impl Renderer {
    pub fn frame_buffer(&self) -> &[u32] {
        bytemuck::cast_slice(self.pixel_buffer.as_ref())
    }

    pub fn copy_vram_fb(&mut self, sx: u16, sy: u16, width: usize, height: usize) {
        let sx = sx as usize;
        let sy = sy as usize;

        for y in 0..height {
            for x in 0..width {
                let vram_addr = 2 * ((sy + y) * 1024 + (sx + x));
                let pixel = u16::from_le_bytes([self.vram[vram_addr], self.vram[vram_addr + 1]]);
                self.pixel_buffer[width * y + x] = Color::new_5bit(pixel);
            }
        }
    }

    pub fn draw_triangle_opaque(&mut self, t: [Vec2; 3], color: u32) {
        let min_x = std::cmp::min(t[0].x, std::cmp::min(t[1].x, t[2].x));
        let min_y = std::cmp::min(t[0].y, std::cmp::min(t[1].y, t[2].y));
        let max_x = std::cmp::max(t[0].x, std::cmp::max(t[1].x, t[2].x));
        let max_y = std::cmp::max(t[0].y, std::cmp::max(t[1].y, t[2].y));

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                if point_in_triangle(t, Vec2::new(x, y)) {
                    let index = CANVAS_WIDTH * (y as usize) + x as usize;
                    self.pixel_buffer[index] = Color::new_8bit(color);
                };
            }
        }
    }

    pub fn draw_rectangle_opaque(&mut self, r: Vec2, side_x: i32, side_y: i32, color: u32) {
        let triangle_half_1 = [
            r + Vec2::new(side_x, 0),
            r + Vec2::new(0, side_y),
            r + Vec2::zero(),
        ];
        let triangle_half_2 = [
            r + Vec2::new(side_x, 0),
            r + Vec2::new(0, side_y),
            r + Vec2::new(side_x, side_y),
        ];

        self.draw_triangle_opaque(triangle_half_1, color);
        self.draw_triangle_opaque(triangle_half_2, color);
    }
}
