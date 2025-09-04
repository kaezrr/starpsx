pub mod utils;
pub mod vec2;

use crate::{
    utils::Color,
    vec2::{Vec2, point_in_triangle},
};

const CANVAS_WIDTH: usize = 800;
const CANVAS_HEIGHT: usize = 800;

pub struct Renderer {
    pub pixel_buffer: Box<[Color; CANVAS_HEIGHT * CANVAS_WIDTH]>,
}

impl Default for Renderer {
    fn default() -> Self {
        let mut render = Self {
            pixel_buffer: Box::new([Color::default(); CANVAS_HEIGHT * CANVAS_WIDTH]),
        };

        let triangle = [
            Vec2::new(50., 50.),
            Vec2::new(200., 250.),
            Vec2::new(100., 70.),
        ];

        render.draw_triangle_opaque(triangle, 0x00ff00);

        render
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

    pub fn draw_triangle_opaque(&mut self, triangle: [Vec2; 3], color: u32) {
        let (min_x, max_x, min_y, max_y) = triangle.iter().fold(
            (
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
            ),
            |(min_x, max_x, min_y, max_y), p| {
                (
                    min_x.min(p.x),
                    max_x.max(p.x),
                    min_y.min(p.y),
                    max_y.max(p.y),
                )
            },
        );

        let min_x = min_x as usize;
        let max_x = max_x as usize;
        let min_y = min_y as usize;
        let max_y = max_y as usize;

        let inside_color = Color::new_8bit(color);
        let outside_color = Color::new_8bit(0x000000);

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let col = if point_in_triangle(triangle, Vec2::new(x as f32, y as f32)) {
                    inside_color
                } else {
                    outside_color
                };
                self.pixel_buffer[CANVAS_WIDTH * y + x] = col;
            }
        }
    }
}
