use crate::vec2::Vec2;

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
        let r = convert_5bit_to_8bit((pixel >> 10) & 0x1F);
        let g = convert_5bit_to_8bit((pixel >> 5) & 0x1F);
        let b = convert_5bit_to_8bit(pixel & 0x1F);
        Self { r, g, b, a: 0 }
    }

    pub fn new_8bit(pixel: u32) -> Self {
        let r = ((pixel >> 16) & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = (pixel & 0xFF) as u8;
        Self { r, g, b, a: 0 }
    }

    pub fn to_5bit(&self) -> u16 {
        let r = (self.r >> 3) as u16;
        let g = (self.g >> 3) as u16;
        let b = (self.b >> 3) as u16;
        r << 10 | g << 5 | b
    }

    pub fn apply_dithering(&mut self, p: Vec2) {
        let offset = DITHER_TABLE[(p.y & 3) as usize][(p.x & 3) as usize];
        self.r = self.r.saturating_add_signed(offset);
        self.g = self.g.saturating_add_signed(offset);
        self.b = self.b.saturating_add_signed(offset);
    }
}

// TODO: Convert to a lookup table later
fn convert_5bit_to_8bit(color: u16) -> u8 {
    (f64::from(color) * 255.0 / 31.0).round() as u8
}

// Store the current drawing context
#[derive(Debug, Default, Clone, Copy)]
pub struct DrawContext {
    pub start_x: usize,
    pub start_y: usize,
    pub width: usize,
    pub height: usize,
    pub dithering: bool,
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
        let base_y = ((data >> 5) & 0x1ff).into();
        Self { base_x, base_y }
    }

    pub fn get_color(&self, vram: &[u8], value: u8) -> u16 {
        let index = 2 * (1024 * self.base_y + self.base_x + value as usize);
        u16::from_be_bytes([vram[index], vram[index + 1]])
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
        let base_y = (((data >> 4) & 1) | ((data >> 10) & 2)).into();
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

    pub fn get_texel(&self, vram: &[u8], p: Vec2) -> Color {
        let val = match self.depth {
            PageColor::Bit4 => self.get_texel_4bit(vram, p),
            PageColor::Bit8 => self.get_texel_8bit(vram, p),
            PageColor::Bit15 => self.get_texel_16bit(vram, p),
        };
        Color::new_5bit(val)
    }

    fn get_texel_16bit(&self, vram: &[u8], p: Vec2) -> u16 {
        let (u, v) = (p.x as usize, p.y as usize);
        let index = 2 * ((self.page_y + v) * 1024 + self.page_x + u);
        u16::from_le_bytes([vram[index], vram[index + 1]])
    }

    fn get_texel_8bit(&self, vram: &[u8], p: Vec2) -> u16 {
        let (u, v) = (p.x as usize, p.y as usize);
        let index = 2 * ((self.page_y + v) * 1024 + self.page_x + u / 2);
        let texel = u16::from_le_bytes([vram[index], vram[index + 1]]);
        let clut_value = (texel >> ((u % 2) * 8)) & 0xFF;
        self.clut.get_color(vram, clut_value as u8)
    }

    fn get_texel_4bit(&self, vram: &[u8], p: Vec2) -> u16 {
        let (u, v) = (p.x as usize, p.y as usize);
        let index = 2 * ((self.page_y + v) * 1024 + self.page_x + u / 4);
        let texel = u16::from_le_bytes([vram[index], vram[index + 1]]);
        let clut_value = (texel >> ((u % 4) * 4)) & 0xF;
        self.clut.get_color(vram, clut_value as u8)
    }
}
