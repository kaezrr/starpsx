pub const fn mask_region(addr: u32) -> u32 {
    addr & match addr >> 29 {
        0b000..=0b011 => 0xFFFFFFFF, // KUSEG
        0b100 => 0x7FFFFFFF,         // KSEG0
        0b101 => 0x1FFFFFFF,         // KSEG1
        0b110 | 0b111 => 0xFFFFFFFF, // KSEG2
        _ => unreachable!(),
    }
}

pub trait ByteAddressable: Copy {
    const LEN: usize;

    type Bytes: for<'a> TryFrom<&'a [u8], Error: core::fmt::Debug> + AsRef<[u8]>;

    fn from_le_bytes(bytes: Self::Bytes) -> Self;

    fn to_le_bytes(self) -> Self::Bytes;

    fn from_u32(val: u32) -> Self;

    fn to_u32(self) -> u32;
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
        }
    };
}

int_impl!(u8);
int_impl!(u16);
int_impl!(u32);
