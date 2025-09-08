pub mod utils;
pub mod vec2;

use crate::{
    utils::{Color, DrawContext, Texture, interpolate_color, interpolate_uv},
    vec2::{Vec2, compute_barycentric_coords, needs_vertex_reordering, point_in_triangle},
};

const VRAM_WIDTH: usize = 1024;
const VRAM_HEIGHT: usize = 512;
const VRAM_SIZE: usize = VRAM_WIDTH * VRAM_HEIGHT;

pub struct Renderer {
    pub pixel_buffer: Box<[Color; VRAM_SIZE]>,
    pub ctx: DrawContext,
    vram: Box<[u16; VRAM_SIZE]>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            ctx: DrawContext::default(),
            vram: Box::new([0; VRAM_SIZE]),
            pixel_buffer: Box::new([Color::default(); VRAM_SIZE]),
        }
    }
}

impl Renderer {
    pub fn vram_read(&self, x: usize, y: usize) -> u16 {
        let index = VRAM_WIDTH * y + x;
        self.vram[index]
    }

    pub fn vram_write(&mut self, x: usize, y: usize, data: u16) {
        let index = VRAM_WIDTH * y + x;
        self.vram[index] = data;
    }

    pub fn vram_self_copy(
        &mut self,
        src_x: usize,
        src_y: usize,
        dst_x: usize,
        dst_y: usize,
        width: usize,
        height: usize,
    ) {
        for y in 0..height {
            let src_row_start = (src_y + y) * VRAM_WIDTH + src_x;
            let dst_row_start = (dst_y + y) * VRAM_WIDTH + dst_x;
            self.vram
                .copy_within(src_row_start..src_row_start + width, dst_row_start);
        }
    }

    pub fn frame_buffer(&self) -> &[u32] {
        bytemuck::cast_slice(self.pixel_buffer.as_ref())
    }

    pub fn copy_vram_to_fb(&mut self) {
        let sx = 0;
        let sy = 0;
        let width = 1024;
        let height = 512;
        // let sx = self.ctx.display_vram_start.x as usize;
        // let sy = self.ctx.display_vram_start.y as usize;
        //
        // let width = self.ctx.resolution.x as usize;
        // let height = self.ctx.resolution.y as usize;

        for y in 0..height {
            for x in 0..width {
                let pixel = self.vram_read(sx + x, sy + y);
                self.pixel_buffer[width * y + x] = Color::new_5bit(pixel);
            }
        }
    }

    // BEWARE OF OFF BY ONE ERRORS!!!
    pub fn draw_rectangle_mono(&mut self, r: Vec2, side_x: i32, side_y: i32, color: u16) {
        let min_x = r.x;
        let min_y = r.y;
        let max_x = r.x + side_x - 1;
        let max_y = r.y + side_y - 1;

        let min_x = std::cmp::max(min_x, self.ctx.drawing_area_top_left.x);
        let min_y = std::cmp::max(min_y, self.ctx.drawing_area_top_left.y);
        let max_x = std::cmp::min(max_x, self.ctx.drawing_area_bottom_right.x);
        let max_y = std::cmp::min(max_y, self.ctx.drawing_area_bottom_right.y);

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                self.vram_write(x as usize, y as usize, color);
            }
        }
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

    pub fn draw_line_poly_mono(&mut self, ls: Vec<Vec2>, color: u16, trans: bool) {
        for i in 1..ls.len() {
            self.draw_line_mono([ls[i - 1], ls[i]], color, trans);
        }
    }

    pub fn draw_line_poly_shaded(&mut self, ls: Vec<Vec2>, color: Vec<u16>, trans: bool) {
        for i in 1..ls.len() {
            self.draw_line_shaded([ls[i - 1], ls[i]], [color[i - 1], color[i]], trans);
        }
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

        let min_x = std::cmp::max(min_x, self.ctx.drawing_area_top_left.x);
        let min_y = std::cmp::max(min_y, self.ctx.drawing_area_top_left.y);
        let max_x = std::cmp::min(max_x, self.ctx.drawing_area_bottom_right.x);
        let max_y = std::cmp::min(max_y, self.ctx.drawing_area_bottom_right.y);

        let colors = colors.map(Color::new_5bit);

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let p = Vec2::new(x, y);
                if let Some(weights) = compute_barycentric_coords(t, p) {
                    let mut color = interpolate_color(weights, colors);
                    if self.ctx.dithering {
                        color.apply_dithering(p);
                    }
                    self.vram_write(x as usize, y as usize, color.to_5bit());
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

        let min_x = std::cmp::max(min_x, self.ctx.drawing_area_top_left.x);
        let min_y = std::cmp::max(min_y, self.ctx.drawing_area_top_left.y);
        let max_x = std::cmp::min(max_x, self.ctx.drawing_area_bottom_right.x);
        let max_y = std::cmp::min(max_y, self.ctx.drawing_area_bottom_right.y);

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let p = Vec2::new(x, y);
                if let Some(weights) = compute_barycentric_coords(t, p) {
                    let uv = interpolate_uv(weights, uvs);
                    let mut color = tex.get_texel(self, uv);

                    // Fully black pixels are ignored in textures
                    if color.to_5bit() == 0 {
                        continue;
                    }

                    if self.ctx.dithering {
                        color.apply_dithering(p);
                    }
                    self.vram_write(x as usize, y as usize, color.to_5bit());
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

        let min_x = std::cmp::max(min_x, self.ctx.drawing_area_top_left.x);
        let min_y = std::cmp::max(min_y, self.ctx.drawing_area_top_left.y);
        let max_x = std::cmp::min(max_x, self.ctx.drawing_area_bottom_right.x);
        let max_y = std::cmp::min(max_y, self.ctx.drawing_area_bottom_right.y);

        let color = Color::new_5bit(color);
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let p = Vec2::new(x, y);
                if point_in_triangle(t, p) {
                    let mut color = color;
                    if self.ctx.dithering {
                        color.apply_dithering(p);
                    }
                    self.vram_write(x as usize, y as usize, color.to_5bit());
                };
            }
        }
    }

    pub fn draw_line_mono(&mut self, l: [Vec2; 2], color: u16, trans: bool) {
        let (x0, x1) = (l[0].x, l[1].x);
        let (y0, y1) = (l[0].y, l[1].y);

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();

        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx + dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            let mut color = Color::new_5bit(color);
            if trans {
                let old = self.vram_read(x as usize, y as usize);
                color.blend(Color::new_5bit(old), self.ctx.transparency_weights);
            }
            self.vram_write(x as usize, y as usize, color.to_5bit());
            let e2 = 2 * err;
            if e2 >= dy {
                if x == x1 {
                    break;
                }
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                if y == y1 {
                    break;
                }
                err += dx;
                y += sy;
            }
        }
    }

    pub fn draw_line_shaded(&mut self, l: [Vec2; 2], colors: [u16; 2], trans: bool) {
        let (x0, x1) = (l[0].x, l[1].x);
        let (y0, y1) = (l[0].y, l[1].y);

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();

        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx + dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            let t = if dx >= -dy {
                f64::from(x - x0) / f64::from(dx)
            } else {
                f64::from(y - y0) / f64::from(-dy)
            };
            let mut color = Color::lerp(
                Color::new_5bit(colors[0]),
                Color::new_5bit(colors[1]),
                t.abs(),
            );
            if trans {
                let old = self.vram_read(x as usize, y as usize);
                color.blend(Color::new_5bit(old), self.ctx.transparency_weights);
            }
            self.vram_write(x as usize, y as usize, color.to_5bit());
            let e2 = 2 * err;
            if e2 >= dy {
                if x == x1 {
                    break;
                }
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                if y == y1 {
                    break;
                }
                err += dx;
                y += sy;
            }
        }
    }
}
