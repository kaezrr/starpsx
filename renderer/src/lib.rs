pub mod utils;
pub mod vec2;

use crate::{
    utils::{Color, DrawContext},
    vec2::{Vec2, point_in_triangle},
};

const VRAM_WIDTH: usize = 1024;
const VRAM_HEIGHT: usize = 512;

pub struct Renderer {
    pub pixel_buffer: Box<[Color; VRAM_HEIGHT * VRAM_WIDTH]>,
    pub vram: Box<[u8; 512 * 2048]>,
    pub ctx: DrawContext,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            ctx: DrawContext::default(),
            vram: Box::new([0; 1024 * 1024]),
            pixel_buffer: Box::new([Color::default(); VRAM_HEIGHT * VRAM_WIDTH]),
        }
        // let mut renderer = Self {
        //     ctx: DrawContext {
        //         start_x: 0,
        //         start_y: 0,
        //         width: 1024,
        //         height: 512,
        //     },
        //     vram: Box::new([0; 1024 * 1024]),
        //     pixel_buffer: Box::new([Color::default(); VRAM_HEIGHT * VRAM_WIDTH]),
        // };
        //
        // renderer.draw_rectangle_opaque(Vec2::zero(), 160, 80, 0x7c00);
        // renderer.draw_rectangle_opaque(Vec2::new(0, 80), 160, 80, 0x03e0);
        // renderer.draw_rectangle_opaque(Vec2::new(0, 160), 160, 80, 0x001f);
        // renderer.copy_vram_to_fb();
        //
        // renderer
    }
}

impl Renderer {
    pub fn frame_buffer(&self) -> &[u32] {
        bytemuck::cast_slice(self.pixel_buffer.as_ref())
    }

    pub fn copy_vram_to_fb(&mut self) {
        for y in 0..self.ctx.height {
            for x in 0..self.ctx.width {
                let index = 2 * ((self.ctx.start_y + y) * VRAM_WIDTH + (self.ctx.start_x + x));
                let pixel = u16::from_le_bytes([self.vram[index], self.vram[index + 1]]);
                self.pixel_buffer[self.ctx.width * y + x] = Color::new_5bit(pixel);
            }
        }
    }

    pub fn draw_triangle_opaque(&mut self, t: [Vec2; 3], color: u16) {
        let min_x = std::cmp::min(t[0].x, std::cmp::min(t[1].x, t[2].x));
        let min_y = std::cmp::min(t[0].y, std::cmp::min(t[1].y, t[2].y));
        let max_x = std::cmp::max(t[0].x, std::cmp::max(t[1].x, t[2].x));
        let max_y = std::cmp::max(t[0].y, std::cmp::max(t[1].y, t[2].y));

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                if point_in_triangle(t, Vec2::new(x, y)) {
                    let index = 2 * (VRAM_WIDTH * (y as usize) + (x as usize));
                    *self.vram[index..].first_chunk_mut().unwrap() = color.to_le_bytes();
                };
            }
        }
    }

    pub fn draw_rectangle_opaque(&mut self, r: Vec2, side_x: i32, side_y: i32, color: u16) {
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
