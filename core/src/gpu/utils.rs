use num_enum::FromPrimitive;
use num_enum::IntoPrimitive;
use starpsx_renderer::utils::Clut;
use starpsx_renderer::utils::Texture;
use starpsx_renderer::vec2::Vec2;

use super::ArrayVec;
use super::Command;
use super::Gpu;

/// Texture color bits per pixel
#[derive(IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum TextureDepth {
    #[default]
    T4 = 0,
    T8 = 1,
    T15 = 2,
}

/// Video modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum VMode {
    #[default]
    Ntsc = 0,
    Pal = 1,
}

/// Requested DMA direction
#[derive(IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum DmaDirection {
    #[default]
    Off = 0,
    Fifo = 1,
    CpuToGpu = 2,
    VRamToCpu = 3,
}

/// Video output horizontal resolution
#[derive(Debug, Clone, Copy, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum HorizontalRes {
    #[default]
    X256 = 0,
    X320 = 1,
    X512 = 2,
    X640 = 3,
    #[num_enum(alternatives = [5, 6, 7])]
    X368 = 4,
}

impl HorizontalRes {
    pub const fn as_value(self) -> u16 {
        match self {
            Self::X256 => 256,
            Self::X320 => 320,
            Self::X368 => 368,
            Self::X512 => 512,
            Self::X640 => 640,
        }
    }
}

/// Video output vertical resolution
#[derive(Debug, Clone, Copy, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum VerticalRes {
    #[default]
    Y240 = 0,
    Y480 = 1,
}

impl VerticalRes {
    pub const fn as_value(self) -> u16 {
        match self {
            Self::Y240 => 240,
            Self::Y480 => 480,
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

pub type CommandFn = fn(&mut Gpu, &[Command]) -> GP0State;
pub type PolyLineFn = fn(&mut Gpu, Vec<u32>, &[u32]) -> GP0State;

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

    pub const fn done(&self) -> bool {
        self.params.len() == self.target_len
    }

    pub fn call(self, gpu: &mut Gpu) -> GP0State {
        (self.func)(gpu, &self.params)
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

        if data & 0xF000_F000 == 0x5000_5000 {
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
        (self.func)(gpu, self.vertices, &self.colors)
    }

    pub const fn done(&self) -> bool {
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
pub const fn parse_xy(data: u32) -> Vec2 {
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

pub const fn parse_uv(data: u32) -> Vec2 {
    let u = data & 0xFF;
    let v = (data >> 8) & 0xFF;
    Vec2::new(u as i32, v as i32)
}
