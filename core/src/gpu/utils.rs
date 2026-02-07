use starpsx_renderer::{
    utils::{Clut, Texture},
    vec2::Vec2,
};

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

/// Video output horizontal resolution
#[derive(Debug, Clone, Copy)]
pub enum HorizontalRes {
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

impl HorizontalRes {
    pub fn as_value(&self) -> u16 {
        match self {
            HorizontalRes::X256 => 256,
            HorizontalRes::X320 => 320,
            HorizontalRes::X368 => 368,
            HorizontalRes::X512 => 512,
            HorizontalRes::X640 => 640,
        }
    }
}

/// Video output vertical resolution
#[derive(Debug, Clone, Copy)]
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

impl VerticalRes {
    pub fn as_value(&self) -> u16 {
        match self {
            VerticalRes::Y240 => 240,
            VerticalRes::Y480 => 480,
        }
    }
}

#[derive(Clone, Copy)]
pub struct VramCopyFields {
    pub vram_x: u16,
    pub vram_y: u16,
    pub width: u16,
    pub height: u16,
    pub current_row: u16,
    pub current_col: u16,
}

pub type CommandFn = fn(&mut Gpu, ArrayVec<Command, 16>) -> GP0State;
pub type PolyLineFn = fn(&mut Gpu, Vec<u32>, Vec<u32>) -> GP0State;

pub struct CommandArguments {
    func: CommandFn,
    params: ArrayVec<Command, 16>,
    target_len: usize,
}

impl CommandArguments {
    pub fn new(func: CommandFn, target_len: usize) -> Self {
        Self {
            func,
            params: ArrayVec::new(),
            target_len,
        }
    }

    pub fn push(&mut self, data: Command) {
        self.params.push(data);
    }

    pub fn done(&self) -> bool {
        self.params.len() == self.target_len
    }

    pub fn call(self, gpu: &mut Gpu) -> GP0State {
        (self.func)(gpu, self.params)
    }
}

pub struct PolyLineArguments {
    func: PolyLineFn,
    vertices: Vec<u32>,
    colors: Vec<u32>,
    shaded: bool,
    done: bool,
}

impl PolyLineArguments {
    pub fn new(func: PolyLineFn, color: bool) -> Self {
        Self {
            func,
            vertices: Vec::new(),
            colors: Vec::new(),
            shaded: color,
            done: false,
        }
    }

    pub fn push(&mut self, data: u32) {
        if self.done {
            return;
        }

        if data & 0xF000F000 == 0x50005000 {
            self.done = true;
            return;
        }

        let needs_color =
            self.colors.is_empty() || (self.shaded && self.colors.len() <= self.vertices.len());

        if needs_color {
            self.colors.push(data);
        } else {
            self.vertices.push(data);
        }
    }

    pub fn call(self, gpu: &mut Gpu) -> GP0State {
        (self.func)(gpu, self.vertices, self.colors)
    }

    pub fn done(&self) -> bool {
        self.done
    }
}

pub enum GP0State {
    AwaitCommand,
    AwaitArgs(CommandArguments),
    CopyToVram(VramCopyFields),
    CopyFromVram(VramCopyFields),
    PolyLine(PolyLineArguments),
}

// Parses YYYYXXXX to x and y vertex coordinates. Both are 11 bit signed numbers
pub fn parse_xy(data: u32) -> Vec2 {
    let x = ((data & 0xFFFF) as i16) << 5 >> 5;
    let y = (((data >> 16) & 0xFFFF) as i16) << 5 >> 5;

    Vec2::new(x as i32, y as i32)
}

pub fn parse_clut_uv(data: u32) -> (Clut, Vec2) {
    let uv = parse_uv(data);
    let clut = Clut::new((data >> 16) as u16);
    (clut, uv)
}

pub fn parse_page_uv(data: u32, clut: Clut) -> (Texture, Vec2) {
    let uv = parse_uv(data);
    let texpage = Texture::new((data >> 16) as u16, Some(clut));
    (texpage, uv)
}

pub fn parse_uv(data: u32) -> Vec2 {
    let u = data & 0xFF;
    let v = (data >> 8) & 0xFF;
    Vec2::new(u as i32, v as i32)
}
