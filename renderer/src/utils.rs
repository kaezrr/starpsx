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
