use crate::{Renderer, vec2::Vec2};

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

const DITHER_TABLE: [[i8; 4]; 4] = [
    [-4, 0, -3, 1],
    [2, -2, 3, -1],
    [-3, 1, -4, 0],
    [3, -1, 2, -2],
];

impl Color {
    pub fn new_5bit(pixel: u16) -> Self {
        let r = convert_5bit_to_8bit(pixel & 0x1F);
        let g = convert_5bit_to_8bit((pixel >> 5) & 0x1F);
        let b = convert_5bit_to_8bit((pixel >> 10) & 0x1F);

        Self { r, g, b, a: 0 }
    }

    pub fn new_8bit(pixel: u32) -> Self {
        let r = (pixel & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = ((pixel >> 16) & 0xFF) as u8;

        Self { r, g, b, a: 0 }
    }

    pub fn to_5bit(&self) -> u16 {
        let r = (self.r >> 3) as u16;
        let g = (self.g >> 3) as u16;
        let b = (self.b >> 3) as u16;

        b << 10 | g << 5 | r
    }

    pub fn apply_dithering(&mut self, p: Vec2) {
        let offset = DITHER_TABLE[(p.y & 3) as usize][(p.x & 3) as usize];

        self.r = self.r.saturating_add_signed(offset);
        self.g = self.g.saturating_add_signed(offset);
        self.b = self.b.saturating_add_signed(offset);
    }

    pub fn blend(&mut self, back: Color, weights: (f64, f64)) {
        let b = (f64::from(back.r), f64::from(back.g), f64::from(back.b));
        let f = (f64::from(self.r), f64::from(self.g), f64::from(self.b));

        self.r = (b.0 * weights.0 + f.0 * weights.1).round() as u8;
        self.g = (b.1 * weights.0 + f.1 * weights.1).round() as u8;
        self.b = (b.2 * weights.0 + f.2 * weights.1).round() as u8;
    }

    pub fn lerp(a: Color, b: Color, t: f64) -> Self {
        let a = (f64::from(a.r), f64::from(a.g), f64::from(a.b));
        let b = (f64::from(b.r), f64::from(b.g), f64::from(b.b));

        let r = (a.0 * (1.0 - t) + b.0 * t).round() as u8;
        let g = (a.1 * (1.0 - t) + b.1 * t).round() as u8;
        let b = (a.2 * (1.0 - t) + b.2 * t).round() as u8;

        Self { r, g, b, a: 0 }
    }
}

// TODO: Convert to a lookup table later
fn convert_5bit_to_8bit(color: u16) -> u8 {
    (f64::from(color) * 255.0 / 31.0).round() as u8
}

// Store the current drawing context
#[derive(Debug, Default, Clone, Copy)]
pub struct DrawContext {
    pub drawing_area_top_left: Vec2,
    pub drawing_area_bottom_right: Vec2,
    pub drawing_area_offset: Vec2,

    pub texture_rect_x_flip: bool,
    pub texture_rect_y_flip: bool,
    pub dithering: bool,
    pub transparency_weights: (f64, f64),

    pub texture_window_mask: Vec2,
    pub texture_window_offset: Vec2,

    pub display_vram_start: Vec2,
    pub display_hori_range: Vec2,
    pub display_line_range: Vec2,

    pub resolution: Vec2,
}

impl DrawContext {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub fn interpolate_color(weights: [f64; 3], colors: [Color; 3]) -> Color {
    let colr = colors.map(|color| f64::from(color.r));
    let colg = colors.map(|color| f64::from(color.g));
    let colb = colors.map(|color| f64::from(color.b));

    let r = (weights[0] * colr[0] + weights[1] * colr[1] + weights[2] * colr[2]).round() as u8;
    let g = (weights[0] * colg[0] + weights[1] * colg[1] + weights[2] * colg[2]).round() as u8;
    let b = (weights[0] * colb[0] + weights[1] * colb[1] + weights[2] * colb[2]).round() as u8;

    Color { r, g, b, a: 0 }
}

pub fn interpolate_uv(weights: [f64; 3], uvs: [Vec2; 3]) -> Vec2 {
    let us = uvs.map(|uv| f64::from(uv.x));
    let vs = uvs.map(|uv| f64::from(uv.y));

    let x = (weights[0] * us[0] + weights[1] * us[1] + weights[2] * us[2]).round() as i32;
    let y = (weights[0] * vs[0] + weights[1] * vs[1] + weights[2] * vs[2]).round() as i32;

    Vec2 { x, y }
}

#[derive(Debug, Clone, Copy)]
pub struct Clut {
    base_x: usize,
    base_y: usize,
}

impl Clut {
    pub fn new(data: u16) -> Self {
        let base_x = ((data & 0x3F) << 4).into();
        let base_y = ((data >> 6) & 0x1ff).into();
        Self { base_x, base_y }
    }

    pub fn get_color(&self, renderer: &Renderer, index: u8) -> u16 {
        renderer.vram_read(self.base_x + index as usize, self.base_y)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PageColor {
    Bit4,
    Bit8,
    Bit15,
}

#[derive(Debug, Clone, Copy)]
pub struct Texture {
    page_x: usize,
    page_y: usize,
    depth: PageColor,
    clut: Clut,
}

impl Texture {
    pub fn new(data: u16, clut: Clut) -> Self {
        let base_x = ((data & 0xF) << 6).into();
        let base_y = (((data >> 4) & 1) << 8).into();
        let depth = match (data >> 6) & 3 {
            0 => PageColor::Bit4,
            1 => PageColor::Bit8,
            2 => PageColor::Bit15,
            _ => unreachable!(),
        };
        Self {
            page_x: base_x,
            page_y: base_y,
            depth,
            clut,
        }
    }

    pub fn get_texel(&self, renderer: &Renderer, p: Vec2) -> Color {
        let val = match self.depth {
            PageColor::Bit4 => self.get_texel_4bit(renderer, p),
            PageColor::Bit8 => self.get_texel_8bit(renderer, p),
            PageColor::Bit15 => self.get_texel_16bit(renderer, p),
        };
        Color::new_5bit(val)
    }

    fn get_texel_16bit(&self, renderer: &Renderer, p: Vec2) -> u16 {
        let (u, v) = (p.x as usize, p.y as usize);
        renderer.vram_read(self.page_x + u, self.page_y + v)
    }

    fn get_texel_8bit(&self, renderer: &Renderer, p: Vec2) -> u16 {
        let (u, v) = (p.x as usize, p.y as usize);
        let texel = renderer.vram_read(self.page_x + u / 2, self.page_y + v);
        let clut_index = (texel >> ((u % 2) * 8)) & 0xFF;

        self.clut.get_color(renderer, clut_index as u8)
    }

    fn get_texel_4bit(&self, renderer: &Renderer, p: Vec2) -> u16 {
        let (u, v) = (p.x as usize, p.y as usize);
        let texel = renderer.vram_read(self.page_x + u / 4, self.page_y + v);
        let clut_index = (texel >> ((u % 4) * 4)) & 0xF;

        self.clut.get_color(renderer, clut_index as u8)
    }
}
