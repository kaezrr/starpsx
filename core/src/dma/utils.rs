#[derive(PartialEq, Clone, Copy)]
pub enum Direction {
    ToRam,
    FromRam,
}

impl From<u8> for Direction {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::ToRam,
            1 => Self::FromRam,
            _ => unreachable!(),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum Step {
    Increment,
    Decrement,
}

impl From<u8> for Step {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Increment,
            1 => Self::Decrement,
            _ => unreachable!(),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum Sync {
    Manual,
    Request,
    LinkedList,
}

impl From<u8> for Sync {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Manual,
            1 => Self::Request,
            2 => Self::LinkedList,
            _ => unreachable!("Unknown sync mode {v}"),
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Port {
    // Macroblock decoder input
    MdecIn,
    // Macroblock decoder output
    MdecOut,
    // Graphics Processing Unit
    Gpu,
    // CD-ROM Drive
    CdRom,
    // Sound Processing Unit
    Spu,
    // Extension Port
    Pio,
    // Clear ordering table
    Otc,
}

impl From<u32> for Port {
    fn from(index: u32) -> Self {
        match index {
            0 => Port::MdecIn,
            1 => Port::MdecOut,
            2 => Port::Gpu,
            3 => Port::CdRom,
            4 => Port::Spu,
            5 => Port::Pio,
            6 => Port::Otc,
            _ => unreachable!("Unknown port {index}"),
        }
    }
}
