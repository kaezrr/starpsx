use crate::{Renderer, vec2::Vec2};

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    mask: u8,
}

const DITHER_TABLE: [[i8; 4]; 4] = [
    [-4, 0, -3, 1],
    [2, -2, 3, -1],
    [-3, 1, -4, 0],
    [3, -1, 2, -2],
];

pub trait From5Bit {
    fn to_color(self) -> Color;
}

impl From5Bit for u16 {
    fn to_color(self) -> Color {
        let r = convert_5bit_to_8bit(self & 0x1F);
        let g = convert_5bit_to_8bit((self >> 5) & 0x1F);
        let b = convert_5bit_to_8bit((self >> 10) & 0x1F);
        let mask = (self >> 15) as u8;
        Color { r, g, b, mask }
    }
}

impl From5Bit for u32 {
    fn to_color(self) -> Color {
        let r = (self & 0xFF) as u16;
        let g = ((self >> 8) & 0xFF) as u16;
        let b = ((self >> 16) & 0xFF) as u16;

        let r = convert_5bit_to_8bit(r >> 3);
        let g = convert_5bit_to_8bit(g >> 3);
        let b = convert_5bit_to_8bit(b >> 3);

        Color { r, g, b, mask: 0 }
    }
}

impl Color {
    pub fn new_5bit<T: From5Bit>(pixel: T) -> Self {
        pixel.to_color()
    }

    pub fn is_masked(&self) -> bool {
        self.mask == 1
    }

    pub fn new_8bit(pixel: u32) -> Self {
        let r = (pixel & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = ((pixel >> 16) & 0xFF) as u8;

        Self { r, g, b, mask: 0 }
    }

    pub fn to_5bit(&self, mask: Option<bool>) -> u16 {
        let r = (self.r >> 3) as u16;
        let g = (self.g >> 3) as u16;
        let b = (self.b >> 3) as u16;
        let m = mask.unwrap_or(self.mask == 1) as u16;

        m << 15 | b << 10 | g << 5 | r
    }

    pub fn apply_dithering(&mut self, p: Vec2) {
        let offset = DITHER_TABLE[(p.y & 3) as usize][(p.x & 3) as usize];

        self.r = self.r.saturating_add_signed(offset);
        self.g = self.g.saturating_add_signed(offset);
        self.b = self.b.saturating_add_signed(offset);
    }

    pub fn blend_screen(&mut self, back: Color, weights: (f64, f64)) {
        let b = (f64::from(back.r), f64::from(back.g), f64::from(back.b));
        let f = (f64::from(self.r), f64::from(self.g), f64::from(self.b));

        self.r = (b.0 * weights.0 + f.0 * weights.1).round() as u8;
        self.g = (b.1 * weights.0 + f.1 * weights.1).round() as u8;
        self.b = (b.2 * weights.0 + f.2 * weights.1).round() as u8;
    }

    pub fn blend(&mut self, poly: Color) {
        let b = (f64::from(poly.r), f64::from(poly.g), f64::from(poly.b));
        let f = (f64::from(self.r), f64::from(self.g), f64::from(self.b));

        self.r = ((b.0 * f.0) / 128.0).round() as u8;
        self.g = ((b.1 * f.1) / 128.0).round() as u8;
        self.b = ((b.2 * f.2) / 128.0).round() as u8;
    }

    pub fn lerp(a: Color, b: Color, t: f64) -> Self {
        let a = (f64::from(a.r), f64::from(a.g), f64::from(a.b));
        let b = (f64::from(b.r), f64::from(b.g), f64::from(b.b));

        let r = (a.0 * (1.0 - t) + b.0 * t).round() as u8;
        let g = (a.1 * (1.0 - t) + b.1 * t).round() as u8;
        let b = (a.2 * (1.0 - t) + b.2 * t).round() as u8;

        Self { r, g, b, mask: 0 }
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

    pub dithering: bool,
    pub transparency_weights: (f64, f64),

    pub texture_window_mask: Vec2,
    pub texture_window_offset: Vec2,

    pub display_vram_start: Vec2,
    pub display_hori_range: Vec2,
    pub display_line_range: Vec2,

    pub resolution_x: HorizontalRes,
    pub resolution_y: VerticalRes,

    pub rect_texture: Texture,

    pub preserve_masked_pixels: bool,
    pub force_set_masked_bit: bool,

    pub display_depth: DisplayDepth,

    pub display_disabled: bool,
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

    Color { r, g, b, mask: 0 }
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

#[derive(Debug, Clone, Copy, Default)]
pub enum PageColor {
    #[default]
    Bit4,
    Bit8,
    Bit15,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Texture {
    page_x: usize,
    page_y: usize,
    depth: PageColor,
    clut: Option<Clut>,
}

impl Texture {
    pub fn new(data: u16, clut: Option<Clut>) -> Self {
        let base_x = ((data & 0xF) << 6).into();
        let base_y = (((data >> 4) & 1) << 8).into();
        let depth = match (data >> 7) & 3 {
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

    pub fn set_clut(&mut self, clut: Clut) {
        self.clut = Some(clut);
    }

    pub fn get_texel(&self, renderer: &Renderer, p: Vec2) -> u16 {
        let Vec2 {
            x: x_mask,
            y: y_mask,
        } = renderer.ctx.texture_window_mask;
        let Vec2 {
            x: x_offset,
            y: y_offset,
        } = renderer.ctx.texture_window_offset;

        // Calculate new texcoords based on some offsets and masks
        let p = Vec2::new(
            (p.x & (!(x_mask * 8))) | ((x_offset & x_mask) * 8),
            (p.y & (!(y_mask * 8))) | ((y_offset & y_mask) * 8),
        );

        match self.depth {
            PageColor::Bit4 => self.get_texel_4bit(renderer, p),
            PageColor::Bit8 => self.get_texel_8bit(renderer, p),
            PageColor::Bit15 => self.get_texel_16bit(renderer, p),
        }
    }

    fn get_texel_16bit(&self, renderer: &Renderer, p: Vec2) -> u16 {
        let (u, v) = (p.x as usize, p.y as usize);
        renderer.vram_read(self.page_x + u, self.page_y + v)
    }

    fn get_texel_8bit(&self, renderer: &Renderer, p: Vec2) -> u16 {
        let (u, v) = (p.x as usize, p.y as usize);
        let texel = renderer.vram_read(self.page_x + u / 2, self.page_y + v);
        let clut_index = (texel >> ((u % 2) * 8)) & 0xFF;

        self.clut.unwrap().get_color(renderer, clut_index as u8)
    }

    fn get_texel_4bit(&self, renderer: &Renderer, p: Vec2) -> u16 {
        let (u, v) = (p.x as usize, p.y as usize);
        let texel = renderer.vram_read(self.page_x + u / 4, self.page_y + v);
        let clut_index = (texel >> ((u % 4) * 4)) & 0xF;

        self.clut.unwrap().get_color(renderer, clut_index as u8)
    }
}

#[derive(Debug)]
pub enum ColorOptions<const SIZE: usize> {
    Mono(Color),
    Shaded([Color; SIZE]),
}

#[derive(Debug)]
pub struct DrawOptions<const SIZE: usize> {
    pub color: ColorOptions<SIZE>,
    pub transparent: bool,
    pub textured: Option<(Texture, bool, [Vec2; 3])>,
}

impl DrawOptions<3> {
    pub fn swap_first_two_vertex(&mut self) {
        if let ColorOptions::Shaded(ref mut x) = self.color {
            x.swap(0, 1);
        }
        if let Some((_, _, uvs)) = self.textured.as_mut() {
            uvs.swap(0, 1);
        }
    }

    pub fn needs_weights(&self) -> bool {
        matches!(self.color, ColorOptions::Shaded(_)) || self.textured.is_some()
    }
}

/// Display color bits per pixel
#[derive(Default, Debug, Clone, Copy)]
pub enum DisplayDepth {
    #[default]
    D15,
    D24,
}

impl From<u8> for DisplayDepth {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::D15,
            1 => Self::D24,
            _ => unreachable!(),
        }
    }
}

impl From<DisplayDepth> for u8 {
    fn from(v: DisplayDepth) -> Self {
        match v {
            DisplayDepth::D15 => 0,
            DisplayDepth::D24 => 1,
        }
    }
}

/// Video output horizontal resolution
#[derive(Default, Debug, Clone, Copy)]
pub enum HorizontalRes {
    #[default]
    X256,
    X320,
    X368,
    X512,
    X640,
}

impl From<u8> for HorizontalRes {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::X256,
            1 => Self::X320,
            2 => Self::X512,
            3 => Self::X640,
            4..=7 => Self::X368,
            _ => unreachable!(),
        }
    }
}

impl From<HorizontalRes> for usize {
    fn from(value: HorizontalRes) -> Self {
        match value {
            HorizontalRes::X256 => 256,
            HorizontalRes::X320 => 320,
            HorizontalRes::X368 => 368,
            HorizontalRes::X512 => 512,
            HorizontalRes::X640 => 640,
        }
    }
}

/// Video output vertical resolution
#[derive(Default, Debug, Clone, Copy)]
pub enum VerticalRes {
    #[default]
    Y240,
    Y480,
}

impl From<u8> for VerticalRes {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Y240,
            1 => Self::Y480,
            _ => unreachable!(),
        }
    }
}

impl From<VerticalRes> for u8 {
    fn from(v: VerticalRes) -> Self {
        match v {
            VerticalRes::Y240 => 0,
            VerticalRes::Y480 => 1,
        }
    }
}

impl From<VerticalRes> for usize {
    fn from(value: VerticalRes) -> Self {
        match value {
            VerticalRes::Y240 => 240,
            VerticalRes::Y480 => 480,
        }
    }
}
