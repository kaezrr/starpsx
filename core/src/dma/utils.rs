use num_enum::FromPrimitive;
use num_enum::IntoPrimitive;

#[derive(PartialEq, Clone, Copy, Debug, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Direction {
    #[default]
    ToRam = 0,
    FromRam = 1,
}

#[derive(PartialEq, Clone, Copy, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Step {
    #[default]
    Increment = 0,
    Decrement = 1,
}

#[derive(PartialEq, Clone, Copy, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Sync {
    #[default]
    Burst = 0,
    Slice = 1,
    LinkedList = 2,
}

#[derive(PartialEq, Clone, Copy, Debug, FromPrimitive, IntoPrimitive)]
#[repr(u32)]
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
