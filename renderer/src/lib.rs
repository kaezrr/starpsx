pub mod utils;
pub mod vec2;

use crate::{
    utils::{
        Clut, Color, ColorOptions, DrawContext, DrawOptions, interpolate_color, interpolate_uv,
    },
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
            vram: vec![0; VRAM_SIZE].try_into().unwrap(),
            pixel_buffer: vec![Color::default(); VRAM_SIZE].try_into().unwrap(),
        }
    }
}

impl Renderer {
    pub fn vram_read(&self, x: usize, y: usize) -> u16 {
        let index = VRAM_WIDTH * y + x;
        self.vram[index]
    }

    pub fn vram_write(&mut self, x: usize, y: usize, data: u16) {
        if self.ctx.preserve_masked_pixels && (self.vram_read(x, y) & 0x8000) != 0 {
            return;
        }

        let index = VRAM_WIDTH * y + x;
        self.vram[index] = data | (self.ctx.force_set_masked_bit as u16) << 15;
    }

    // Should be affected by mask bit, fix in the future
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

    pub fn copy_display_to_fb(&mut self) {
        let (sx, sy, width, height) = if cfg!(feature = "full-vram") {
            (0, 0, VRAM_WIDTH, VRAM_HEIGHT)
        } else {
            (
                self.ctx.display_vram_start.x as usize,
                self.ctx.display_vram_start.y as usize,
                self.ctx.resolution.x as usize,
                self.ctx.resolution.y as usize,
            )
        };

        match self.ctx.display_depth {
            utils::DisplayDepth::D15 => {
                for y in 0..height {
                    for x in 0..width {
                        // let pixel = self.vram_read(sx + x, sy + y);
                        let index = VRAM_WIDTH * (sy + y) + (sx + x);
                        let pixel = self.vram[index];
                        self.pixel_buffer[width * y + x] = Color::new_5bit(pixel);
                    }
                }
            }
            utils::DisplayDepth::D24 => {
                let mut vram_x = 0;
                for y in 0..height {
                    for x in (0..width).step_by(2) {
                        let w0 = self.vram_read(sx + vram_x, sy + y) as u32;
                        let w1 = self.vram_read(sx + vram_x + 1, sy + y) as u32;
                        let w2 = self.vram_read(sx + vram_x + 2, sy + y) as u32;

                        let pixel0 = w0 | (w1 & 0xFF) << 16;
                        let pixel1 = (w2 << 8) | w1 >> 8;

                        self.pixel_buffer[width * y + x] = Color::new_8bit(pixel0);
                        self.pixel_buffer[width * y + x + 1] = Color::new_8bit(pixel1);

                        vram_x += 3; // 3 words per 2 pixels
                    }
                    vram_x = 0;
                }
            }
        }
    }

    // Don't reuse the rectangle drawer for this because this isn't affected by masked bit
    pub fn vram_quick_fill(&mut self, r: Vec2, side_x: i32, side_y: i32, color: Color) {
        let min_x = r.x;
        let min_y = r.y;
        let max_x = r.x + side_x - 1;
        let max_y = r.y + side_y - 1;

        let min_x = std::cmp::max(min_x, self.ctx.drawing_area_top_left.x) as usize;
        let min_y = std::cmp::max(min_y, self.ctx.drawing_area_top_left.y) as usize;
        let max_x = std::cmp::min(max_x, self.ctx.drawing_area_bottom_right.x) as usize;
        let max_y = std::cmp::min(max_y, self.ctx.drawing_area_bottom_right.y) as usize;

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let index = VRAM_WIDTH * y + x;
                self.vram[index] = color.to_5bit(None);
            }
        }
    }

    // BEWARE OF OFF BY ONE ERRORS!!!
    pub fn draw_rectangle_mono<const SEMI_TRANS: bool>(
        &mut self,
        mut r: Vec2,
        side_x: i32,
        side_y: i32,
        color: Color,
        textured: Option<(Clut, bool, Vec2)>,
    ) {
        r += self.ctx.drawing_area_offset;

        let min_x = r.x;
        let min_y = r.y;
        let max_x = r.x + side_x - 1;
        let max_y = r.y + side_y - 1;

        let min_x = std::cmp::max(min_x, self.ctx.drawing_area_top_left.x);
        let min_y = std::cmp::max(min_y, self.ctx.drawing_area_top_left.y);
        let max_x = std::cmp::min(max_x, self.ctx.drawing_area_bottom_right.x);
        let max_y = std::cmp::min(max_y, self.ctx.drawing_area_bottom_right.y);

        if let Some((clut, _, _)) = textured {
            self.ctx.rect_texture.set_clut(clut);
        }

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let mut color = color;

                if let Some((_, blended, start_uv)) = textured {
                    let uv = start_uv + Vec2::new(x, y) - r;
                    let texel = self.ctx.rect_texture.get_texel(self, uv);
                    // Fully black texels are ignored
                    if texel == 0 {
                        continue;
                    }
                    let mut tex_color = Color::new_5bit(texel);
                    if blended {
                        tex_color.blend(color);
                    }
                    color = tex_color
                }

                if SEMI_TRANS {
                    let old = self.vram_read(x as usize, y as usize);
                    color.blend_screen(Color::new_5bit(old), self.ctx.transparency_weights);
                }

                self.vram_write(
                    x as usize,
                    y as usize,
                    color.to_5bit(
                        self.ctx
                            .force_set_masked_bit
                            .then_some(true)
                            .or_else(|| textured.is_none().then_some(false)),
                    ),
                );
            }
        }
    }

    pub fn draw_line(&mut self, mut l: [Vec2; 2], options: DrawOptions<2>) {
        l.iter_mut()
            .for_each(|v| *v += self.ctx.drawing_area_offset);

        let (x0, x1) = (l[0].x, l[1].x);
        let (y0, y1) = (l[0].y, l[1].y);

        let x0 = x0.clamp(
            self.ctx.drawing_area_top_left.x,
            self.ctx.drawing_area_bottom_right.x,
        );
        let x1 = x1.clamp(
            self.ctx.drawing_area_top_left.x,
            self.ctx.drawing_area_bottom_right.x,
        );
        let y0 = y0.clamp(
            self.ctx.drawing_area_top_left.y,
            self.ctx.drawing_area_bottom_right.y,
        );
        let y1 = y1.clamp(
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
            let mut color = match options.color {
                ColorOptions::Mono(color) => color,
                ColorOptions::Shaded(colors) => {
                    let t = if dx >= -dy {
                        f64::from(x - x0) / f64::from(dx)
                    } else {
                        f64::from(y - y0) / f64::from(-dy)
                    };
                    Color::lerp(colors[0], colors[1], t.abs())
                }
            };

            if options.transparent {
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

    pub fn draw_triangle(&mut self, mut t: [Vec2; 3], mut options: DrawOptions<3>) {
        t.iter_mut()
            .for_each(|v| *v += self.ctx.drawing_area_offset);

        if needs_vertex_reordering(&t) {
            t.swap(0, 1);
            options.swap_first_two_vertex();
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
                if !point_in_triangle(t, p) {
                    continue;
                }

                let weights = options
                    .needs_weights()
                    .then(|| compute_barycentric_coords(t, p));

                let mut color = match options.color {
                    ColorOptions::Mono(color) => color,
                    ColorOptions::Shaded(colors) => interpolate_color(weights.unwrap(), colors),
                };

                if let Some((texture, blended, uvs)) = options.textured {
                    let uv = interpolate_uv(weights.unwrap(), uvs);
                    let texel = texture.get_texel(self, uv);
                    // Fully black texels are ignored
                    if texel == 0 {
                        continue;
                    }
                    let mut tex_color = Color::new_5bit(texel);
                    if blended {
                        tex_color.blend(color);
                    }
                    color = tex_color
                }

                if options.transparent {
                    let old = self.vram_read(x as usize, y as usize);
                    color.blend_screen(Color::new_5bit(old), self.ctx.transparency_weights);
                }

                if self.ctx.dithering {
                    color.apply_dithering(p);
                }

                self.vram_write(
                    x as usize,
                    y as usize,
                    color.to_5bit(
                        self.ctx
                            .force_set_masked_bit
                            .then_some(true)
                            .or_else(|| options.textured.is_none().then_some(false)),
                    ),
                );
            }
        }
    }
}
