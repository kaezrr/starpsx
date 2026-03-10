pub mod utils;
pub mod vec2;

use crate::utils::Color;
use crate::utils::DrawContext;
use crate::utils::RectTextureOptions;
use crate::utils::TextureOptions;
use crate::vec2::Vec2;
use crate::vec2::edge_function;
use crate::vec2::is_top_left;
use crate::vec2::needs_vertex_reordering;

const VRAM_WIDTH: usize = 1024;
const VRAM_HEIGHT: usize = 512;
const VRAM_SIZE: usize = VRAM_WIDTH * VRAM_HEIGHT;

pub struct Renderer {
    pub ctx: DrawContext,

    vram: Box<[u16; VRAM_SIZE]>,
    frame: FrameBuffer,
}

#[derive(Clone)]
pub struct FrameBuffer {
    pub rgba: Vec<Color>,

    /// Resolution in pixels
    pub resolution: [usize; 2],
    pub is_interlaced: bool,
}

impl FrameBuffer {
    /// Create a width x height framebuffer fully black
    fn new(width: usize, height: usize, is_interlaced: bool) -> Self {
        Self {
            rgba: vec![Color::BLACK; width * height],
            is_interlaced,

            // Interlaced frames have duplicated rows
            resolution: [width, height],
        }
    }

    /// A fully black 1x1 framebuffer
    fn black() -> Self {
        Self {
            rgba: vec![Color::BLACK],
            resolution: [1, 1],
            is_interlaced: false,
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            ctx: DrawContext::default(),
            vram: vec![0; VRAM_SIZE].try_into().unwrap(),

            frame: FrameBuffer::black(),
        }
    }
}

impl Renderer {
    pub fn change_resolution(&mut self, width: u16, height: u16) {
        // Dont do anything if frame buffer size didnt change
        if width == self.ctx.display_width && height == self.ctx.display_height {
            return;
        }

        self.ctx.display_width = width;
        self.ctx.display_height = height;

        let width = width as usize;
        let height = height as usize * if self.ctx.interlaced { 1 } else { 2 };

        // Replace the frame buffer because resolution changed
        self.frame = FrameBuffer::new(width, height, self.ctx.interlaced);
    }

    pub fn vram_read(&self, x: usize, y: usize) -> u16 {
        let index = VRAM_WIDTH * y + x;
        self.vram[index]
    }

    pub fn vram_write(&mut self, x: usize, y: usize, data: u16) {
        let index = VRAM_WIDTH * y + x;
        if self.ctx.preserve_masked_pixels && (self.vram[index] & 0x8000) != 0 {
            return;
        }

        self.vram[index] = data | (self.ctx.force_set_masked_bit as u16) << 15;
    }

    pub fn vram_self_copy(&mut self, src: Vec2, dst: Vec2, size: Vec2) {
        let width = size.x as usize;
        let height = size.y as usize;

        for y in 0..height {
            for x in 0..width {
                let pixel = self.vram_read(src.x as usize + x, src.y as usize + y);
                self.vram_write(dst.x as usize + x, dst.y as usize + y, pixel);
            }
        }
    }

    pub fn produce_frame_buffer(&mut self) -> FrameBuffer {
        // Display is disabled or resolution is invalid
        if self.ctx.display_disabled {
            return FrameBuffer::black();
        }

        let (sx, sy, width, height, interlaced) = (
            self.ctx.display_vram_start.x as usize,
            self.ctx.display_vram_start.y as usize,
            self.ctx.display_width as usize,
            self.ctx.display_height as usize,
            self.ctx.interlaced,
        );

        let draw_odd = self.ctx.frame_counter & 1 != 0;

        match self.ctx.display_depth {
            utils::DisplayDepth::D15 => {
                if interlaced {
                    // Draw alternating odd/even lines every frame
                    for y in (draw_odd as usize..height).step_by(2) {
                        for x in 0..width {
                            let pixel = self.vram_read(sx + x, sy + y);
                            let index = y * width + x;
                            self.frame.rgba[index] = Color::new_5bit(pixel).with_full_alpha();
                        }
                    }
                } else {
                    for y in 0..height {
                        let base_y = y * 2;

                        for x in 0..width {
                            let pixel = self.vram_read(sx + x, sy + y);
                            let index = base_y * width + x;
                            self.frame.rgba[index] = Color::new_5bit(pixel).with_full_alpha();
                        }

                        // Non interlaced displays have each row duplicated
                        let start = base_y * width;
                        let end = start + width;
                        self.frame.rgba.copy_within(start..end, end);
                    }
                }
            }
            utils::DisplayDepth::D24 => {
                if interlaced {
                    // Draw alternating odd/even lines every frame
                    for y in (draw_odd as usize..height).step_by(2) {
                        let mut vram_x = 0;

                        // Write one full row
                        for x in (0..width).step_by(2) {
                            let w0 = self.vram_read(sx + vram_x, sy + y) as u32;
                            let w1 = self.vram_read(sx + vram_x + 1, sy + y) as u32;
                            let w2 = self.vram_read(sx + vram_x + 2, sy + y) as u32;

                            let pixel0 = w0 | ((w1 & 0xFF) << 16);
                            let pixel1 = (w2 << 8) | (w1 >> 8);

                            let idx = y * width + x;

                            self.frame.rgba[idx] = Color::new_8bit(pixel0).with_full_alpha();
                            self.frame.rgba[idx + 1] = Color::new_8bit(pixel1).with_full_alpha();

                            vram_x += 3;
                        }
                    }
                } else {
                    for y in 0..height {
                        let base_y = y * 2;
                        let mut vram_x = 0;

                        // Write one full row
                        for x in (0..width).step_by(2) {
                            let w0 = self.vram_read(sx + vram_x, sy + y) as u32;
                            let w1 = self.vram_read(sx + vram_x + 1, sy + y) as u32;
                            let w2 = self.vram_read(sx + vram_x + 2, sy + y) as u32;

                            let pixel0 = w0 | ((w1 & 0xFF) << 16);
                            let pixel1 = (w2 << 8) | (w1 >> 8);

                            let idx = base_y * width + x;

                            self.frame.rgba[idx] = Color::new_8bit(pixel0).with_full_alpha();
                            self.frame.rgba[idx + 1] = Color::new_8bit(pixel1).with_full_alpha();

                            vram_x += 3;
                        }

                        // Non interlaced displays have each row duplicated
                        let start = base_y * width;
                        let end = start + width;
                        self.frame.rgba.copy_within(start..end, end);
                    }
                }
            }
        };

        let vrange = self.ctx.display_ver_range as usize;

        if height >= vrange {
            let starting_row = vrange * width * if interlaced { 1 } else { 2 };
            self.frame.rgba[starting_row..].fill(Color::BLACK);
        }

        self.frame.clone()
    }

    pub fn produce_vram_framebuffer(&self) -> FrameBuffer {
        let (sx, sy, width, height) = (0, 0, VRAM_WIDTH, VRAM_HEIGHT);

        let mut vram_frame = FrameBuffer::new(width, height, false);

        for y in 0..height {
            for x in 0..width {
                let pixel = self.vram_read(sx + x, sy + y);
                let index = y * width + x;
                vram_frame.rgba[index] = Color::new_5bit(pixel).with_full_alpha();
            }
        }

        vram_frame
    }

    // Don't reuse the rectangle drawer for this because this isn't affected by masked bit
    pub fn vram_quick_fill(&mut self, r: Vec2, side_x: i32, side_y: i32, color: Color) {
        let min_x = r.x as usize;
        let min_y = r.y as usize;
        let max_x = (r.x + side_x - 1) as usize;
        let max_y = (r.y + side_y - 1) as usize;

        for x in min_x..max_x + 1 {
            for y in min_y..max_y + 1 {
                let index = VRAM_WIDTH * y + x;
                self.vram[index] = color.to_5bit(None);
            }
        }
    }

    pub fn draw_rectangle<const SEMI_TRANS: bool>(
        &mut self,
        mut r: Vec2,
        side: Vec2,
        color: Color,
    ) {
        r += self.ctx.drawing_area_offset;
        let min_x = r.x.max(self.ctx.drawing_area_top_left.x);
        let min_y = r.y.max(self.ctx.drawing_area_top_left.y);
        let max_x = (r.x + side.x - 1).min(self.ctx.drawing_area_bottom_right.x);
        let max_y = (r.y + side.y - 1).min(self.ctx.drawing_area_bottom_right.y);

        // Out of screen
        if max_x < self.ctx.drawing_area_top_left.x
            || min_x > self.ctx.drawing_area_bottom_right.x
            || max_y < self.ctx.drawing_area_top_left.y
            || min_y > self.ctx.drawing_area_bottom_right.y
        {
            return;
        }

        for y in min_y..max_y + 1 {
            for x in min_x..max_x + 1 {
                let mut color = color;
                if SEMI_TRANS {
                    let old = self.vram_read(x as usize, y as usize);
                    color.blend_screen(Color::new_5bit(old), self.ctx.transparency_weights);
                }
                self.vram_write(
                    x as usize,
                    y as usize,
                    color.to_5bit(Some(self.ctx.force_set_masked_bit)),
                );
            }
        }
    }

    pub fn draw_rectangle_textured<const SEMI_TRANS: bool, const BLEND: bool>(
        &mut self,
        mut r: Vec2,
        side: Vec2,
        color: Color,
        tex: RectTextureOptions,
    ) {
        r += self.ctx.drawing_area_offset;
        let min_x = r.x.max(self.ctx.drawing_area_top_left.x);
        let min_y = r.y.max(self.ctx.drawing_area_top_left.y);
        let max_x = (r.x + side.x - 1).min(self.ctx.drawing_area_bottom_right.x);
        let max_y = (r.y + side.y - 1).min(self.ctx.drawing_area_bottom_right.y);

        // Out of screen
        if max_x < self.ctx.drawing_area_top_left.x
            || min_x > self.ctx.drawing_area_bottom_right.x
            || max_y < self.ctx.drawing_area_top_left.y
            || min_y > self.ctx.drawing_area_bottom_right.y
        {
            return;
        }

        self.ctx.rect_texture.set_clut(tex.clut);

        for y in min_y..max_y + 1 {
            for x in min_x..max_x + 1 {
                let uv = tex.uv + Vec2::new(x, y) - r;
                let texel = self.ctx.rect_texture.get_texel(self, uv);
                if texel == 0 {
                    continue;
                }

                let mut tex_color = Color::new_5bit(texel);
                if BLEND {
                    tex_color.blend(color);
                }

                if SEMI_TRANS && (texel & 0x8000) != 0 {
                    let old = self.vram_read(x as usize, y as usize);
                    tex_color.blend_screen(Color::new_5bit(old), self.ctx.transparency_weights);
                }
                let mask = self.ctx.force_set_masked_bit.then_some(true);
                self.vram_write(x as usize, y as usize, tex_color.to_5bit(mask));
            }
        }
    }

    pub fn draw_line<const SEMI_TRANS: bool>(&mut self, mut l: [Vec2; 2], mono: Color) {
        l[0] += self.ctx.drawing_area_offset;
        l[1] += self.ctx.drawing_area_offset;

        let x0 = l[0].x.clamp(
            self.ctx.drawing_area_top_left.x,
            self.ctx.drawing_area_bottom_right.x,
        );
        let x1 = l[1].x.clamp(
            self.ctx.drawing_area_top_left.x,
            self.ctx.drawing_area_bottom_right.x,
        );
        let y0 = l[0].y.clamp(
            self.ctx.drawing_area_top_left.y,
            self.ctx.drawing_area_bottom_right.y,
        );
        let y1 = l[1].y.clamp(
            self.ctx.drawing_area_top_left.y,
            self.ctx.drawing_area_bottom_right.y,
        );

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();

        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx + dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            let mut color = mono;

            if SEMI_TRANS {
                let old = self.vram_read(x as usize, y as usize);
                color.blend_screen(Color::new_5bit(old), self.ctx.transparency_weights);
            }

            self.vram_write(
                x as usize,
                y as usize,
                color.to_5bit(Some(self.ctx.force_set_masked_bit)),
            );

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

    pub fn draw_line_shaded<const SEMI_TRANS: bool>(
        &mut self,
        mut l: [Vec2; 2],
        shaded: [Color; 2],
    ) {
        l[0] += self.ctx.drawing_area_offset;
        l[1] += self.ctx.drawing_area_offset;

        let x0 = l[0].x.clamp(
            self.ctx.drawing_area_top_left.x,
            self.ctx.drawing_area_bottom_right.x,
        );
        let x1 = l[1].x.clamp(
            self.ctx.drawing_area_top_left.x,
            self.ctx.drawing_area_bottom_right.x,
        );
        let y0 = l[0].y.clamp(
            self.ctx.drawing_area_top_left.y,
            self.ctx.drawing_area_bottom_right.y,
        );
        let y1 = l[1].y.clamp(
            self.ctx.drawing_area_top_left.y,
            self.ctx.drawing_area_bottom_right.y,
        );

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();

        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx + dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            let mut color = {
                let (num, denom) = if dx >= -dy {
                    ((x - x0).abs(), dx)
                } else {
                    ((y - y0).abs(), -dy)
                };

                if denom == 0 {
                    shaded[0]
                } else {
                    let inv = denom - num;
                    let red = (shaded[0].r as i32 * inv + shaded[1].r as i32 * num) / denom;
                    let green = (shaded[0].g as i32 * inv + shaded[1].g as i32 * num) / denom;
                    let blue = (shaded[0].b as i32 * inv + shaded[1].b as i32 * num) / denom;

                    Color {
                        r: red as u8,
                        g: green as u8,
                        b: blue as u8,
                        mask: 0,
                    }
                }
            };

            if SEMI_TRANS {
                let old = self.vram_read(x as usize, y as usize);
                color.blend_screen(Color::new_5bit(old), self.ctx.transparency_weights);
            }

            self.vram_write(
                x as usize,
                y as usize,
                color.to_5bit(Some(self.ctx.force_set_masked_bit)),
            );

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

    pub fn draw_triangle<const SEMI_TRANS: bool>(&mut self, mut t: [Vec2; 3], mono: Color) {
        t[0] += self.ctx.drawing_area_offset;
        t[1] += self.ctx.drawing_area_offset;
        t[2] += self.ctx.drawing_area_offset;

        if needs_vertex_reordering(&t) {
            t.swap(0, 1);
        }

        let min_x = t[0].x.min(t[1].x).min(t[2].x);
        let min_y = t[0].y.min(t[1].y).min(t[2].y);
        let max_x = t[0].x.max(t[1].x).max(t[2].x);
        let max_y = t[0].y.max(t[1].y).max(t[2].y);

        // Out of screen
        if max_x < self.ctx.drawing_area_top_left.x
            || min_x > self.ctx.drawing_area_bottom_right.x
            || max_y < self.ctx.drawing_area_top_left.y
            || min_y > self.ctx.drawing_area_bottom_right.y
        {
            return;
        }

        let min_x = min_x.max(self.ctx.drawing_area_top_left.x) as usize;
        let min_y = min_y.max(self.ctx.drawing_area_top_left.y) as usize;
        let max_x = max_x.min(self.ctx.drawing_area_bottom_right.x) as usize;
        let max_y = max_y.min(self.ctx.drawing_area_bottom_right.y) as usize;

        let start = Vec2::new(min_x as i32, min_y as i32);
        let (mut e1_row, a1, b1) = edge_function(start, t[0], t[1]);
        let (mut e2_row, a2, b2) = edge_function(start, t[1], t[2]);
        let (mut e3_row, a3, b3) = edge_function(start, t[2], t[0]);

        let bias1 = !is_top_left(t[0], t[1]) as i32;
        let bias2 = !is_top_left(t[1], t[2]) as i32;
        let bias3 = !is_top_left(t[2], t[0]) as i32;

        for y in min_y..max_y + 1 {
            let mut e1 = e1_row;
            let mut e2 = e2_row;
            let mut e3 = e3_row;

            for x in min_x..max_x + 1 {
                if e1 >= bias1 && e2 >= bias2 && e3 >= bias3 {
                    let mut color = mono;

                    if SEMI_TRANS {
                        let old = self.vram_read(x, y);
                        color.blend_screen(Color::new_5bit(old), self.ctx.transparency_weights);
                    }

                    if self.ctx.dithering {
                        color.apply_dithering(x, y);
                    }

                    self.vram_write(x, y, color.to_5bit(Some(self.ctx.force_set_masked_bit)));
                }

                e1 += a1;
                e2 += a2;
                e3 += a3;
            }

            e1_row += b1;
            e2_row += b2;
            e3_row += b3;
        }
    }

    pub fn draw_triangle_shaded<const SEMI_TRANS: bool>(
        &mut self,
        mut t: [Vec2; 3],
        mut shaded: [Color; 3],
    ) {
        t[0] += self.ctx.drawing_area_offset;
        t[1] += self.ctx.drawing_area_offset;
        t[2] += self.ctx.drawing_area_offset;

        if needs_vertex_reordering(&t) {
            t.swap(0, 1);
            shaded.swap(0, 1);
        }

        let min_x = t[0].x.min(t[1].x).min(t[2].x);
        let min_y = t[0].y.min(t[1].y).min(t[2].y);
        let max_x = t[0].x.max(t[1].x).max(t[2].x);
        let max_y = t[0].y.max(t[1].y).max(t[2].y);

        // Out of screen
        if max_x < self.ctx.drawing_area_top_left.x
            || min_x > self.ctx.drawing_area_bottom_right.x
            || max_y < self.ctx.drawing_area_top_left.y
            || min_y > self.ctx.drawing_area_bottom_right.y
        {
            return;
        }

        let min_x = min_x.max(self.ctx.drawing_area_top_left.x) as usize;
        let min_y = min_y.max(self.ctx.drawing_area_top_left.y) as usize;
        let max_x = max_x.min(self.ctx.drawing_area_bottom_right.x) as usize;
        let max_y = max_y.min(self.ctx.drawing_area_bottom_right.y) as usize;

        let start = Vec2::new(min_x as i32, min_y as i32);
        let (mut e1_row, a1, b1) = edge_function(start, t[0], t[1]);
        let (mut e2_row, a2, b2) = edge_function(start, t[1], t[2]);
        let (mut e3_row, a3, b3) = edge_function(start, t[2], t[0]);

        let bias1 = !is_top_left(t[0], t[1]) as i32;
        let bias2 = !is_top_left(t[1], t[2]) as i32;
        let bias3 = !is_top_left(t[2], t[0]) as i32;

        let [c0, c1, c2] = shaded;

        let r_dx = a2 * c0.r as i32 + a3 * c1.r as i32 + a1 * c2.r as i32;
        let g_dx = a2 * c0.g as i32 + a3 * c1.g as i32 + a1 * c2.g as i32;
        let b_dx = a2 * c0.b as i32 + a3 * c1.b as i32 + a1 * c2.b as i32;

        let r_dy = b2 * c0.r as i32 + b3 * c1.r as i32 + b1 * c2.r as i32;
        let g_dy = b2 * c0.g as i32 + b3 * c1.g as i32 + b1 * c2.g as i32;
        let b_dy = b2 * c0.b as i32 + b3 * c1.b as i32 + b1 * c2.b as i32;

        let mut r_row = e2_row * c0.r as i32 + e3_row * c1.r as i32 + e1_row * c2.r as i32;
        let mut g_row = e2_row * c0.g as i32 + e3_row * c1.g as i32 + e1_row * c2.g as i32;
        let mut b_row = e2_row * c0.b as i32 + e3_row * c1.b as i32 + e1_row * c2.b as i32;

        let sum = e1_row + e2_row + e3_row;

        for y in min_y..max_y + 1 {
            let mut e1 = e1_row;
            let mut e2 = e2_row;
            let mut e3 = e3_row;
            let mut r_num = r_row;
            let mut g_num = g_row;
            let mut b_num = b_row;

            for x in min_x..max_x + 1 {
                if e1 >= bias1 && e2 >= bias2 && e3 >= bias3 {
                    let mut color = Color {
                        r: (r_num / sum) as u8,
                        g: (g_num / sum) as u8,
                        b: (b_num / sum) as u8,
                        mask: 0,
                    };

                    if SEMI_TRANS {
                        let old = self.vram_read(x, y);
                        color.blend_screen(Color::new_5bit(old), self.ctx.transparency_weights);
                    }

                    if self.ctx.dithering {
                        color.apply_dithering(x, y);
                    }

                    self.vram_write(x, y, color.to_5bit(Some(self.ctx.force_set_masked_bit)));
                }

                e1 += a1;
                e2 += a2;
                e3 += a3;
                r_num += r_dx;
                g_num += g_dx;
                b_num += b_dx;
            }

            e1_row += b1;
            e2_row += b2;
            e3_row += b3;
            r_row += r_dy;
            g_row += g_dy;
            b_row += b_dy;
        }
    }

    pub fn draw_triangle_textured<const SEMI_TRANS: bool, const BLEND: bool>(
        &mut self,
        mut t: [Vec2; 3],
        mono: Color,
        mut tex: TextureOptions,
    ) {
        t[0] += self.ctx.drawing_area_offset;
        t[1] += self.ctx.drawing_area_offset;
        t[2] += self.ctx.drawing_area_offset;

        if needs_vertex_reordering(&t) {
            t.swap(0, 1);
            tex.uvs.swap(0, 1);
        }

        let min_x = t[0].x.min(t[1].x).min(t[2].x);
        let min_y = t[0].y.min(t[1].y).min(t[2].y);
        let max_x = t[0].x.max(t[1].x).max(t[2].x);
        let max_y = t[0].y.max(t[1].y).max(t[2].y);

        // Out of screen
        if max_x < self.ctx.drawing_area_top_left.x
            || min_x > self.ctx.drawing_area_bottom_right.x
            || max_y < self.ctx.drawing_area_top_left.y
            || min_y > self.ctx.drawing_area_bottom_right.y
        {
            return;
        }

        let min_x = min_x.max(self.ctx.drawing_area_top_left.x) as usize;
        let min_y = min_y.max(self.ctx.drawing_area_top_left.y) as usize;
        let max_x = max_x.min(self.ctx.drawing_area_bottom_right.x) as usize;
        let max_y = max_y.min(self.ctx.drawing_area_bottom_right.y) as usize;

        let start = Vec2::new(min_x as i32, min_y as i32);
        let (mut e1_row, a1, b1) = edge_function(start, t[0], t[1]);
        let (mut e2_row, a2, b2) = edge_function(start, t[1], t[2]);
        let (mut e3_row, a3, b3) = edge_function(start, t[2], t[0]);

        let bias1 = !is_top_left(t[0], t[1]) as i32;
        let bias2 = !is_top_left(t[1], t[2]) as i32;
        let bias3 = !is_top_left(t[2], t[0]) as i32;

        let [uv0, uv1, uv2] = tex.uvs;

        let u_dx = a2 * uv0.x + a3 * uv1.x + a1 * uv2.x;
        let v_dx = a2 * uv0.y + a3 * uv1.y + a1 * uv2.y;
        let u_dy = b2 * uv0.x + b3 * uv1.x + b1 * uv2.x;
        let v_dy = b2 * uv0.y + b3 * uv1.y + b1 * uv2.y;

        let mut u_row = e2_row * uv0.x + e3_row * uv1.x + e1_row * uv2.x;
        let mut v_row = e2_row * uv0.y + e3_row * uv1.y + e1_row * uv2.y;

        let sum = e1_row + e2_row + e3_row;

        for y in min_y..max_y + 1 {
            let mut e1 = e1_row;
            let mut e2 = e2_row;
            let mut e3 = e3_row;
            let mut u_num = u_row;
            let mut v_num = v_row;

            for x in min_x..max_x + 1 {
                if e1 >= bias1 && e2 >= bias2 && e3 >= bias3 {
                    let texel = tex.texture.get_texel(
                        self,
                        Vec2 {
                            x: u_num / sum,
                            y: v_num / sum,
                        },
                    );

                    // Fully black texels are ignored
                    if texel != 0 {
                        let mut color = Color::new_5bit(texel);
                        if BLEND {
                            color.blend(mono);
                        }

                        if SEMI_TRANS && (texel & 0x8000) != 0 {
                            let old = self.vram_read(x, y);
                            color.blend_screen(Color::new_5bit(old), self.ctx.transparency_weights);
                        }

                        if self.ctx.dithering {
                            color.apply_dithering(x, y);
                        }

                        self.vram_write(
                            x,
                            y,
                            color.to_5bit(self.ctx.force_set_masked_bit.then_some(true)),
                        );
                    }
                }

                e1 += a1;
                e2 += a2;
                e3 += a3;
                u_num += u_dx;
                v_num += v_dx;
            }

            e1_row += b1;
            e2_row += b2;
            e3_row += b3;
            u_row += u_dy;
            v_row += v_dy;
        }
    }

    pub fn draw_triangle_textured_shaded<const SEMI_TRANS: bool, const BLEND: bool>(
        &mut self,
        mut t: [Vec2; 3],
        mut shaded: [Color; 3],
        mut tex: TextureOptions,
    ) {
        t[0] += self.ctx.drawing_area_offset;
        t[1] += self.ctx.drawing_area_offset;
        t[2] += self.ctx.drawing_area_offset;

        if needs_vertex_reordering(&t) {
            t.swap(0, 1);
            shaded.swap(0, 1);
            tex.uvs.swap(0, 1);
        }

        let min_x = t[0].x.min(t[1].x).min(t[2].x);
        let min_y = t[0].y.min(t[1].y).min(t[2].y);
        let max_x = t[0].x.max(t[1].x).max(t[2].x);
        let max_y = t[0].y.max(t[1].y).max(t[2].y);

        // Out of screen
        if max_x < self.ctx.drawing_area_top_left.x
            || min_x > self.ctx.drawing_area_bottom_right.x
            || max_y < self.ctx.drawing_area_top_left.y
            || min_y > self.ctx.drawing_area_bottom_right.y
        {
            return;
        }

        let min_x = min_x.max(self.ctx.drawing_area_top_left.x) as usize;
        let min_y = min_y.max(self.ctx.drawing_area_top_left.y) as usize;
        let max_x = max_x.min(self.ctx.drawing_area_bottom_right.x) as usize;
        let max_y = max_y.min(self.ctx.drawing_area_bottom_right.y) as usize;

        let start = Vec2::new(min_x as i32, min_y as i32);
        let (mut e1_row, a1, b1) = edge_function(start, t[0], t[1]);
        let (mut e2_row, a2, b2) = edge_function(start, t[1], t[2]);
        let (mut e3_row, a3, b3) = edge_function(start, t[2], t[0]);

        let bias1 = !is_top_left(t[0], t[1]) as i32;
        let bias2 = !is_top_left(t[1], t[2]) as i32;
        let bias3 = !is_top_left(t[2], t[0]) as i32;

        let [c0, c1, c2] = shaded;

        let r_dx = a2 * c0.r as i32 + a3 * c1.r as i32 + a1 * c2.r as i32;
        let g_dx = a2 * c0.g as i32 + a3 * c1.g as i32 + a1 * c2.g as i32;
        let b_dx = a2 * c0.b as i32 + a3 * c1.b as i32 + a1 * c2.b as i32;

        let r_dy = b2 * c0.r as i32 + b3 * c1.r as i32 + b1 * c2.r as i32;
        let g_dy = b2 * c0.g as i32 + b3 * c1.g as i32 + b1 * c2.g as i32;
        let b_dy = b2 * c0.b as i32 + b3 * c1.b as i32 + b1 * c2.b as i32;

        let mut r_row = e2_row * c0.r as i32 + e3_row * c1.r as i32 + e1_row * c2.r as i32;
        let mut g_row = e2_row * c0.g as i32 + e3_row * c1.g as i32 + e1_row * c2.g as i32;
        let mut b_row = e2_row * c0.b as i32 + e3_row * c1.b as i32 + e1_row * c2.b as i32;

        let [uv0, uv1, uv2] = tex.uvs;

        let u_dx = a2 * uv0.x + a3 * uv1.x + a1 * uv2.x;
        let v_dx = a2 * uv0.y + a3 * uv1.y + a1 * uv2.y;
        let u_dy = b2 * uv0.x + b3 * uv1.x + b1 * uv2.x;
        let v_dy = b2 * uv0.y + b3 * uv1.y + b1 * uv2.y;

        let mut u_row = e2_row * uv0.x + e3_row * uv1.x + e1_row * uv2.x;
        let mut v_row = e2_row * uv0.y + e3_row * uv1.y + e1_row * uv2.y;

        let sum = e1_row + e2_row + e3_row;

        for y in min_y..max_y + 1 {
            let mut e1 = e1_row;
            let mut e2 = e2_row;
            let mut e3 = e3_row;
            let mut r_num = r_row;
            let mut g_num = g_row;
            let mut b_num = b_row;
            let mut u_num = u_row;
            let mut v_num = v_row;

            for x in min_x..max_x + 1 {
                if e1 >= bias1 && e2 >= bias2 && e3 >= bias3 {
                    let texel = tex.texture.get_texel(
                        self,
                        Vec2 {
                            x: u_num / sum,
                            y: v_num / sum,
                        },
                    );
                    // Fully black texels are ignored
                    if texel != 0 {
                        let mut color = Color::new_5bit(texel);
                        if BLEND {
                            color.blend(Color {
                                r: (r_num / sum) as u8,
                                g: (g_num / sum) as u8,
                                b: (b_num / sum) as u8,
                                mask: 0,
                            });
                        }

                        if SEMI_TRANS && (texel >> 15) & 1 == 1 {
                            let old = self.vram_read(x, y);
                            color.blend_screen(Color::new_5bit(old), self.ctx.transparency_weights);
                        }

                        if self.ctx.dithering {
                            color.apply_dithering(x, y);
                        }

                        self.vram_write(
                            x,
                            y,
                            color.to_5bit(self.ctx.force_set_masked_bit.then_some(true)),
                        );
                    }
                }

                e1 += a1;
                e2 += a2;
                e3 += a3;
                r_num += r_dx;
                g_num += g_dx;
                b_num += b_dx;
                u_num += u_dx;
                v_num += v_dx;
            }
            e1_row += b1;
            e2_row += b2;
            e3_row += b3;
            r_row += r_dy;
            g_row += g_dy;
            b_row += b_dy;
            u_row += u_dy;
            v_row += v_dy;
        }
    }
}
