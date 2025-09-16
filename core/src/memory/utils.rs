pub mod map {
    pub struct Range(u32, u32);
    impl Range {
        pub fn contains(&self, addr: u32) -> Option<u32> {
            (self.0..self.0 + self.1)
                .contains(&addr)
                .then_some(addr - self.0)
        }
    }

    pub const RAM: Range = Range(0x00000000, 2048 * 1024);
    pub const BIOS: Range = Range(0x1FC00000, 512 * 1024);
    pub const MEMCTL: Range = Range(0x1F801000, 36);
    pub const RAMSIZE: Range = Range(0x1F801060, 4);
    pub const SPU: Range = Range(0x1F801C00, 640);
    pub const CACHECTL: Range = Range(0xFFFE0130, 4);
    pub const EXPANSION2: Range = Range(0x1F802000, 66);
    pub const EXPANSION1: Range = Range(0x1F000000, 0x100);
    pub const IRQCTL: Range = Range(0x1F801070, 8);
    pub const TIMERS: Range = Range(0x1F801100, 48);
    pub const DMA: Range = Range(0x1F801080, 0x80);
    pub const GPU: Range = Range(0x1F801810, 8);
    pub const SCRATCH: Range = Range(0x1F800000, 0x400);
    pub const PERIPHERAL: Range = Range(0x1F801040, 0x20);
}

pub const fn mask_region(addr: u32) -> u32 {
    addr & match addr >> 29 {
        0b000..=0b011 => 0xFFFFFFFF, // KUSEG
        0b100 => 0x7FFFFFFF,         // KSEG0
        0b101 => 0x1FFFFFFF,         // KSEG1
        0b110 | 0b111 => 0xFFFFFFFF, // KSEG2
        _ => unreachable!(),
    }
}

pub trait ByteAddressable {
    const LEN: usize;
    type Bytes: for<'a> TryFrom<&'a [u8], Error: core::fmt::Debug> + AsRef<[u8]>;
    fn from_le_bytes(bytes: Self::Bytes) -> Self;
    fn to_le_bytes(self) -> Self::Bytes;
    fn from_u32(val: u32) -> Self;
    fn to_u32(self) -> u32;
    fn zeroed() -> Self;
}

macro_rules! int_impl {
    ($int:ty) => {
        impl ByteAddressable for $int {
            const LEN: usize = size_of::<Self>();
            type Bytes = [u8; Self::LEN];
            fn from_le_bytes(bytes: Self::Bytes) -> Self {
                <$int>::from_le_bytes(bytes)
            }
            fn to_le_bytes(self) -> Self::Bytes {
                self.to_le_bytes()
            }
            fn from_u32(val: u32) -> Self {
                val as Self
            }
            fn to_u32(self) -> u32 {
                self as u32
            }
            fn zeroed() -> Self {
                unsafe { std::mem::zeroed() }
            }
        }
    };
}

int_impl!(u8);
int_impl!(u16);
int_impl!(u32);
