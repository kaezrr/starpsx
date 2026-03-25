use num_enum::FromPrimitive;
use num_enum::IntoPrimitive;

use crate::Renderer;
use crate::vec2::Vec2;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,

    /// Bit 15: mask bit
    pub m: u8,
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
        let m = (self >> 15) as u8;
        Color { r, g, b, m }
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

        Color { r, g, b, m: 0 }
    }
}

impl Color {
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        m: 0xFF,
    };

    pub fn new_5bit<T: From5Bit>(pixel: T) -> Self {
        pixel.to_color()
    }

    #[must_use]
    pub const fn with_full_alpha(mut self) -> Self {
        self.m = 0xFF;
        self
    }

    #[must_use]
    pub const fn is_masked(&self) -> bool {
        self.m == 1
    }

    #[must_use]
    pub const fn new_8bit(pixel: u32) -> Self {
        let r = (pixel & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = ((pixel >> 16) & 0xFF) as u8;

        Self { r, g, b, m: 0 }
    }

    #[must_use]
    pub fn to_5bit(&self, mask: Option<bool>) -> u16 {
        let r = u16::from(self.r >> 3);
        let g = u16::from(self.g >> 3);
        let b = u16::from(self.b >> 3);
        let m = u16::from(mask.unwrap_or(self.m == 1));

        m << 15 | b << 10 | g << 5 | r
    }

    pub const fn apply_dithering(&mut self, x: usize, y: usize) {
        let offset = DITHER_TABLE[y & 3][x & 3];

        self.r = self.r.saturating_add_signed(offset);
        self.g = self.g.saturating_add_signed(offset);
        self.b = self.b.saturating_add_signed(offset);
    }

    // weights is a 30.2 fixed point number
    pub fn blend_screen(&mut self, back: Self, weights: (i32, i32)) {
        let (w0, w1) = weights;
        self.r = ((i32::from(back.r) * w0 + i32::from(self.r) * w1) >> 2).clamp(0, 255) as u8;
        self.g = ((i32::from(back.g) * w0 + i32::from(self.g) * w1) >> 2).clamp(0, 255) as u8;
        self.b = ((i32::from(back.b) * w0 + i32::from(self.b) * w1) >> 2).clamp(0, 255) as u8;
    }

    pub fn blend(&mut self, poly: Self) {
        self.r = ((u16::from(self.r) * u16::from(poly.r)) >> 7).min(255) as u8;
        self.g = ((u16::from(self.g) * u16::from(poly.g)) >> 7).min(255) as u8;
        self.b = ((u16::from(self.b) * u16::from(poly.b)) >> 7).min(255) as u8;
    }
}

const FIVE_BIT_TO_8BIT: [u8; 32] = {
    let mut table = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        table[i] = (i as f64 * 255.0 / 31.0).round() as u8;
        i += 1;
    }
    table
};

const fn convert_5bit_to_8bit(color: u16) -> u8 {
    FIVE_BIT_TO_8BIT[color as usize]
}

// Store the current drawing context
#[derive(Debug, Default)]
pub struct DrawContext {
    pub drawing_area_top_left: Vec2,
    pub drawing_area_bottom_right: Vec2,
    pub drawing_area_offset: Vec2,

    pub dithering: bool,
    pub transparency_weights: (i32, i32),

    pub texture_window_mask: Vec2,
    pub texture_window_offset: Vec2,

    pub display_vram_start: Vec2,

    pub display_width: u16,
    pub display_height: u16,

    pub display_hor_range: u16,
    pub display_ver_range: u16,

    pub rect_texture: Texture,

    pub preserve_masked_pixels: bool,
    pub force_set_masked_bit: bool,

    pub display_depth: DisplayDepth,

    pub display_disabled: bool,
    pub interlaced: bool,

    pub frame_counter: u32,
    pub line_counter: u32,
}

impl DrawContext {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Clut {
    base_x: usize,
    base_y: usize,
}

impl Clut {
    #[must_use]
    pub fn new(data: u16) -> Self {
        let base_x = ((data & 0x3F) << 4).into();
        let base_y = ((data >> 6) & 0x1ff).into();
        Self { base_x, base_y }
    }

    #[must_use]
    pub fn get_color(&self, renderer: &Renderer, index: u8) -> u16 {
        renderer.vram_read(self.base_x + index as usize, self.base_y)
    }
}

#[derive(Debug, Clone, Copy, Default, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum PageColor {
    #[default]
    Bit4 = 0,
    Bit8 = 1,
    #[num_enum(alternatives = [3])]
    Bit15 = 2,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Texture {
    page_x: usize,
    page_y: usize,
    depth: PageColor,
    clut: Option<Clut>,
}

impl Texture {
    #[must_use]
    pub fn new(data: u16, clut: Option<Clut>) -> Self {
        let base_x = ((data & 0xF) << 6).into();
        let base_y = (((data >> 4) & 1) << 8).into();
        let depth = PageColor::from(((data >> 7) & 3) as u8);
        Self {
            page_x: base_x,
            page_y: base_y,
            depth,
            clut,
        }
    }

    pub const fn set_clut(&mut self, clut: Clut) {
        self.clut = Some(clut);
    }

    #[must_use]
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

        self.clut
            .expect("8 bit texture must have color table")
            .get_color(renderer, clut_index as u8)
    }

    fn get_texel_4bit(&self, renderer: &Renderer, p: Vec2) -> u16 {
        let (u, v) = (p.x as usize, p.y as usize);
        let texel = renderer.vram_read(self.page_x + u / 4, self.page_y + v);
        let clut_index = (texel >> ((u % 4) * 4)) & 0xF;

        self.clut
            .expect("4 bit texture must have color table")
            .get_color(renderer, clut_index as u8)
    }
}

pub struct TextureOptions {
    pub texture: Texture,
    pub uvs: [Vec2; 3],
}

pub struct RectTextureOptions {
    pub clut: Clut,
    pub uv: Vec2,
}

/// Display color bits per pixel
#[derive(Default, Debug, Clone, Copy, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum DisplayDepth {
    #[default]
    D15 = 0,
    D24 = 1,
}
