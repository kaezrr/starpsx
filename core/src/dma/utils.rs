use num_enum::FromPrimitive;

#[derive(PartialEq, Clone, Copy, Debug)]
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

#[derive(PartialEq, Clone, Copy, Debug, FromPrimitive)]
#[repr(usize)]
pub enum Port {
    // Macroblock decoder input
    #[default]
    MdecIn = 0,
    // Macroblock decoder output
    MdecOut = 1,
    // Graphics Processing Unit
    Gpu = 2,
    // CD-ROM Drive
    CdRom = 3,
    // Sound Processing Unit
    Spu = 4,
    // Extension Port
    Pio = 5,
    // Clear ordering table
    Otc = 6,
}
