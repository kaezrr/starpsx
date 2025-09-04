use super::*;

/// Texture color bits per pixel
pub enum TextureDepth {
    T4,
    T8,
    T15,
}

impl From<u8> for TextureDepth {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::T4,
            1 => Self::T8,
            2 => Self::T15,
            _ => unreachable!(),
        }
    }
}

impl From<TextureDepth> for u8 {
    fn from(v: TextureDepth) -> Self {
        match v {
            TextureDepth::T4 => 0,
            TextureDepth::T8 => 1,
            TextureDepth::T15 => 2,
        }
    }
}

/// Interlaced output splits frames into 2 fields (top = odd lines, bottom = even lines)
pub enum Field {
    Top,
    Bottom,
}

impl From<u8> for Field {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Bottom,
            1 => Self::Top,
            _ => unreachable!(),
        }
    }
}

impl From<Field> for u8 {
    fn from(v: Field) -> Self {
        match v {
            Field::Bottom => 0,
            Field::Top => 1,
        }
    }
}

/// Video output horizontal resolution
pub enum HorizontalRes {
    X256,
    X320,
    X512,
    X368,
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

/// Video output vertical resolution
pub enum VerticalRes {
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
/// Video modes
pub enum VMode {
    Ntsc,
    Pal,
}

impl From<u8> for VMode {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Ntsc,
            1 => Self::Pal,
            _ => unreachable!(),
        }
    }
}

impl From<VMode> for u8 {
    fn from(v: VMode) -> Self {
        match v {
            VMode::Ntsc => 0,
            VMode::Pal => 1,
        }
    }
}

/// Display color bits per pixel
pub enum DisplayDepth {
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

/// Requested DMA direction
pub enum DmaDirection {
    Off,
    Fifo,
    CpuToGpu,
    VRamToCpu,
}

impl From<u8> for DmaDirection {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Off,
            1 => Self::Fifo,
            2 => Self::CpuToGpu,
            3 => Self::VRamToCpu,
            _ => unreachable!(),
        }
    }
}

impl From<DmaDirection> for u8 {
    fn from(v: DmaDirection) -> Self {
        match v {
            DmaDirection::Off => 0,
            DmaDirection::Fifo => 1,
            DmaDirection::CpuToGpu => 2,
            DmaDirection::VRamToCpu => 3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VramCopyFields {
    pub vram_x: u16,
    pub vram_y: u16,
    pub width: u16,
    pub height: u16,
    pub current_row: u16,
    pub current_col: u16,
}

#[derive(Debug)]
pub enum GP0State {
    AwaitCommand,
    AwaitArgs { cmd: fn(&mut Gpu), len: usize },
    CopyToVram(VramCopyFields),
}

pub fn bgr_to_rgb16(data: u16) -> u16 {
    let r = data & 0x1F;
    let g = (data >> 5) & 0x1F;
    let b = (data >> 10) & 0x1F;
    r << 10 | g << 5 | b
}

pub fn parse_color_16(data: u32) -> u16 {
    let r = (data & 0xFF) >> 3;
    let g = ((data >> 8) & 0xFF) >> 3;
    let b = ((data >> 16) & 0xFF) >> 3;
    (r << 10 | g << 5 | b) as u16
}

pub fn parse_x_y(data: u32) -> (u32, u32) {
    let x = data & 0x3FF;
    let y = (data >> 16) & 0x1FF;
    (x, y)
}
