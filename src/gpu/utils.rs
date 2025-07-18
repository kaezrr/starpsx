/// Texture color bits per pixel
pub enum TextureDepth {
    T4Bit,
    T8Bit,
    T15Bit,
}

impl From<u8> for TextureDepth {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::T4Bit,
            1 => Self::T4Bit,
            2 => Self::T4Bit,
            _ => unreachable!(),
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

/// Video output vertical resolution
pub enum VerticalRes {
    Y240Lines,
    Y480Lines,
}

impl From<u8> for VerticalRes {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Y240Lines,
            1 => Self::Y480Lines,
            _ => unreachable!(),
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

/// Display color bits per pixel
pub enum DisplayDepth {
    D15Bits,
    D24Bits,
}

impl From<u8> for DisplayDepth {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::D15Bits,
            1 => Self::D24Bits,
            _ => unreachable!(),
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
