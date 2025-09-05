pub mod utils;
pub mod vec2;

use crate::{
    utils::{Color, DrawContext, Texture, interpolate_color, interpolate_uv},
    vec2::{Vec2, compute_barycentric_coords, needs_vertex_reordering, point_in_triangle},
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
        // let quad = [
        //     Vec2::new(400, 100),
        //     Vec2::new(600, 300),
        //     Vec2::new(200, 300),
        //     Vec2::new(400, 500),
        // ];
        // let color = [0x7c00, 0x03e0, 0x001f, 0x7c1f];
        // renderer.draw_quad_shaded_opaque(quad, color);
        // renderer.copy_vram_to_fb();
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

    pub fn draw_rectangle_mono(&mut self, r: Vec2, side_x: i32, side_y: i32, color: u16) {
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

        self.draw_triangle_mono_opaque(triangle_half_1, color);
        self.draw_triangle_mono_opaque(triangle_half_2, color);
    }

    pub fn draw_quad_mono_opaque(&mut self, q: [Vec2; 4], color: u16) {
        let triangle_half_1 = [q[0], q[1], q[2]];
        let triangle_half_2 = [q[1], q[2], q[3]];

        self.draw_triangle_mono_opaque(triangle_half_1, color);
        self.draw_triangle_mono_opaque(triangle_half_2, color);
    }

    pub fn draw_quad_shaded_opaque(&mut self, q: [Vec2; 4], colors: [u16; 4]) {
        let triangle_half_1 = [q[0], q[1], q[2]];
        let triangle_half_2 = [q[1], q[2], q[3]];

        self.draw_triangle_shaded_opaque(triangle_half_1, colors[..3].try_into().unwrap());
        self.draw_triangle_shaded_opaque(triangle_half_2, colors[1..].try_into().unwrap());
    }

    pub fn draw_quad_texture_blend_opaque(&mut self, q: [Vec2; 4], uvs: [Vec2; 4], tex: Texture) {
        let triangle_half_1 = [q[0], q[1], q[2]];
        let triangle_half_2 = [q[1], q[2], q[3]];

        self.draw_triangle_texture_opaque(triangle_half_1, uvs[..3].try_into().unwrap(), tex);
        self.draw_triangle_texture_opaque(triangle_half_2, uvs[1..].try_into().unwrap(), tex);
    }

    pub fn draw_triangle_shaded_opaque(&mut self, mut t: [Vec2; 3], mut colors: [u16; 3]) {
        if needs_vertex_reordering(&t) {
            t.swap(0, 1);
            colors.swap(0, 1);
        }
        let min_x = std::cmp::min(t[0].x, std::cmp::min(t[1].x, t[2].x));
        let min_y = std::cmp::min(t[0].y, std::cmp::min(t[1].y, t[2].y));
        let max_x = std::cmp::max(t[0].x, std::cmp::max(t[1].x, t[2].x));
        let max_y = std::cmp::max(t[0].y, std::cmp::max(t[1].y, t[2].y));

        let min_x = std::cmp::max(min_x, self.ctx.start_x as i32);
        let min_y = std::cmp::max(min_y, self.ctx.start_y as i32);
        let max_x = std::cmp::min(max_x, (self.ctx.start_x + self.ctx.width - 1) as i32);
        let max_y = std::cmp::min(max_y, (self.ctx.start_y + self.ctx.height - 1) as i32);

        let colors = colors.map(Color::new_5bit);

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let p = Vec2::new(x, y);
                if let Some(weights) = compute_barycentric_coords(t, p) {
                    let index = 2 * (VRAM_WIDTH * (y as usize) + (x as usize));
                    let mut color = interpolate_color(weights, colors);
                    if self.ctx.dithering {
                        color.apply_dithering(p);
                    }
                    *self.vram[index..].first_chunk_mut().unwrap() = color.to_5bit().to_le_bytes();
                };
            }
        }
    }

    pub fn draw_triangle_texture_opaque(
        &mut self,
        mut t: [Vec2; 3],
        mut uvs: [Vec2; 3],
        tex: Texture,
    ) {
        if needs_vertex_reordering(&t) {
            t.swap(0, 1);
            uvs.swap(0, 1);
        }

        let min_x = std::cmp::min(t[0].x, std::cmp::min(t[1].x, t[2].x));
        let min_y = std::cmp::min(t[0].y, std::cmp::min(t[1].y, t[2].y));
        let max_x = std::cmp::max(t[0].x, std::cmp::max(t[1].x, t[2].x));
        let max_y = std::cmp::max(t[0].y, std::cmp::max(t[1].y, t[2].y));

        let min_x = std::cmp::max(min_x, self.ctx.start_x as i32);
        let min_y = std::cmp::max(min_y, self.ctx.start_y as i32);
        let max_x = std::cmp::min(max_x, (self.ctx.start_x + self.ctx.width - 1) as i32);
        let max_y = std::cmp::min(max_y, (self.ctx.start_y + self.ctx.height - 1) as i32);

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let p = Vec2::new(x, y);
                if let Some(weights) = compute_barycentric_coords(t, p) {
                    let index = 2 * (VRAM_WIDTH * (y as usize) + (x as usize));
                    let uv = interpolate_uv(weights, uvs);
                    let color = tex.get_texel(self.vram.as_ref(), uv);
                    *self.vram[index..].first_chunk_mut().unwrap() = color.to_5bit().to_le_bytes();
                };
            }
        }
    }

    pub fn draw_triangle_mono_opaque(&mut self, mut t: [Vec2; 3], color: u16) {
        if needs_vertex_reordering(&t) {
            t.swap(0, 1);
        }
        let min_x = std::cmp::min(t[0].x, std::cmp::min(t[1].x, t[2].x));
        let min_y = std::cmp::min(t[0].y, std::cmp::min(t[1].y, t[2].y));
        let max_x = std::cmp::max(t[0].x, std::cmp::max(t[1].x, t[2].x));
        let max_y = std::cmp::max(t[0].y, std::cmp::max(t[1].y, t[2].y));

        let min_x = std::cmp::max(min_x, self.ctx.start_x as i32);
        let min_y = std::cmp::max(min_y, self.ctx.start_y as i32);
        let max_x = std::cmp::min(max_x, (self.ctx.start_x + self.ctx.width - 1) as i32);
        let max_y = std::cmp::min(max_y, (self.ctx.start_y + self.ctx.height - 1) as i32);

        let color = Color::new_5bit(color);
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let p = Vec2::new(x, y);
                if point_in_triangle(t, p) {
                    let index = 2 * (VRAM_WIDTH * (y as usize) + (x as usize));
                    *self.vram[index..].first_chunk_mut().unwrap() = color.to_5bit().to_le_bytes();
                };
            }
        }
    }
}
