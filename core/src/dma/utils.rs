use num_enum::FromPrimitive;

#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive)]
#[repr(u8)]
pub enum Direction {
    #[default]
    ToRam,
    FromRam,
}

#[derive(PartialEq, Eq, Clone, Copy, FromPrimitive)]
#[repr(u8)]
pub enum Step {
    #[default]
    Increment,
    Decrement,
}

#[derive(PartialEq, Eq, Clone, Copy, FromPrimitive)]
#[repr(u8)]
pub enum Mode {
    #[default]
    Burst,
    Slice,
    LinkedList,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive)]
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
