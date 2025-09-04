#[repr(C)]
#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

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
}

// TODO: Convert to a lookup table later
fn convert_5bit_to_8bit(color: u16) -> u8 {
    (f64::from(color) * 255.0 / 31.0).round() as u8
}
