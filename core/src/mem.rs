use crate::cpu::utils::Exception;
use crate::{System, cdrom, sio};
use crate::{dma, gpu, irq, timers};
use std::error::Error;
use std::fmt::{Display, LowerHex};

pub trait ByteAddressable: Copy + LowerHex + Display {
    const LEN: usize;

    type Bytes: for<'a> TryFrom<&'a [u8], Error: core::fmt::Debug> + AsRef<[u8]>;

    fn from_le_bytes(bytes: Self::Bytes) -> Self;

    fn to_le_bytes(self) -> Self::Bytes;

    fn from_u32(val: u32) -> Self;

    fn to_u32(self) -> u32;

    fn to_u16(self) -> u16;

    fn to_u8(self) -> u8;
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

            fn to_u16(self) -> u16 {
                self as u16
            }

            fn to_u8(self) -> u8 {
                self as u8
            }
        }
    };
}

int_impl!(u8);
int_impl!(u16);
int_impl!(u32);

pub mod bios {
    use super::*;
    pub const PADDR_START: u32 = 0x1FC00000;
    pub const PADDR_END: u32 = 0x1FC7FFFF;

    pub struct Bios {
        bytes: Box<[u8; 512 * 1024]>,
    }

    impl Bios {
        pub fn build(bios_path: &String) -> Result<Self, Box<dyn Error>> {
            let bytes = match std::fs::read(bios_path)?.try_into() {
                Ok(data) => data,
                Err(_) => return Err("invalid bios file.".into()),
            };

            Ok(Bios {
                bytes: Box::new(bytes),
            })
        }

        pub fn read<T: ByteAddressable>(&self, addr: u32) -> T {
            let addr = (addr - PADDR_START) as usize;
            T::from_le_bytes(self.bytes[addr..addr + size_of::<T>()].try_into().unwrap())
        }
    }
}

pub mod ram {
    use super::*;
    pub const PADDR_START: u32 = 0x00000000;
    pub const PADDR_END: u32 = 0x001FFFFF;

    pub struct Ram {
        pub bytes: Box<[u8; 0x200000]>,
    }

    impl Default for Ram {
        fn default() -> Self {
            Self {
                bytes: Box::new([0; 0x200000]),
            }
        }
    }

    impl Ram {
        pub fn read<T: ByteAddressable>(&self, addr: u32) -> T {
            let addr = addr as usize;
            T::from_le_bytes(self.bytes[addr..addr + T::LEN].try_into().unwrap())
        }

        pub fn write<T: ByteAddressable>(&mut self, addr: u32, val: T) {
            let addr = addr as usize;
            self.bytes[addr..addr + T::LEN].copy_from_slice(val.to_le_bytes().as_ref());
        }
    }
}

pub mod scratch {
    use super::*;
    pub const PADDR_START: u32 = 0x1F800000;
    pub const PADDR_END: u32 = 0x1F8003FF;

    pub struct Scratch {
        bytes: Box<[u8; 0x400]>,
    }

    impl Default for Scratch {
        fn default() -> Self {
            Self {
                bytes: Box::new([0; 0x400]),
            }
        }
    }

    impl Scratch {
        pub fn read<T: ByteAddressable>(&self, addr: u32) -> T {
            let addr = (addr - PADDR_START) as usize;
            T::from_le_bytes(self.bytes[addr..addr + size_of::<T>()].try_into().unwrap())
        }

        pub fn write<T: ByteAddressable>(&mut self, addr: u32, val: T) {
            let addr = (addr - PADDR_START) as usize;
            self.bytes[addr..addr + size_of::<T>()].copy_from_slice(val.to_le_bytes().as_ref());
        }
    }
}

macro_rules! stubbed {
    ($region:expr) => {{
        eprintln!($region);
        T::from_u32(0)
    }};
}

const fn mask_region(addr: u32) -> u32 {
    addr & match addr >> 29 {
        0b000..=0b011 => 0xFFFFFFFF, // KUSEG
        0b100 => 0x7FFFFFFF,         // KSEG0
        0b101 => 0x1FFFFFFF,         // KSEG1
        0b110 | 0b111 => 0xFFFFFFFF, // KSEG2
        _ => unreachable!(),
    }
}

impl System {
    pub fn read<T: ByteAddressable>(&mut self, addr: u32) -> Result<T, Exception> {
        if !addr.is_multiple_of(T::LEN as u32) {
            return Err(Exception::LoadAddressError(addr));
        }

        let addr = mask_region(addr);

        let data = match addr {
            ram::PADDR_START..=ram::PADDR_END => self.ram.read(addr),

            bios::PADDR_START..=bios::PADDR_END => self.bios.read(addr),

            scratch::PADDR_START..=scratch::PADDR_END => self.scratch.read(addr),

            gpu::PADDR_START..=gpu::PADDR_END => gpu::read(self, addr),

            dma::PADDR_START..=dma::PADDR_END => dma::read(self, addr),

            irq::PADDR_START..=irq::PADDR_END => irq::read(self, addr),

            timers::PADDR_START..=timers::PADDR_END => timers::read(self, addr),

            cdrom::PADDR_START..=cdrom::PADDR_END => cdrom::read(self, addr),

            sio::PADDR_START..=sio::PADDR_END => sio::read(self, addr),

            0x1F801000..=0x1F801023 => stubbed!("Unhandled read to memctl"),

            0x1F801060..=0x1F801063 => unimplemented!("read to ramsize"),

            0x1F801C00..=0x1F801E7F => stubbed!("Unhandled read to the SPU"),

            0xFFFE0130..=0xFFFE0133 => unimplemented!("read to cachectl"),

            0x1F000000..=0x1F0000FF => stubbed!("Unhandled read to the expansion1"),

            0x1F802000..=0x1F802041 => unimplemented!("read to expansion2"),

            _ => unimplemented!("read at {addr:#08X}"),
        };

        Ok(data)
    }

    pub fn write<T: ByteAddressable>(&mut self, addr: u32, data: T) -> Result<(), Exception> {
        if !addr.is_multiple_of(T::LEN as u32) {
            return Err(Exception::StoreAddressError(addr));
        }
        let addr = mask_region(addr);

        match addr {
            ram::PADDR_START..=ram::PADDR_END => self.ram.write(addr, data),

            scratch::PADDR_START..=scratch::PADDR_END => self.scratch.write(addr, data),

            gpu::PADDR_START..=gpu::PADDR_END => gpu::write(self, addr, data),

            dma::PADDR_START..=dma::PADDR_END => dma::write(self, addr, data),

            irq::PADDR_START..=irq::PADDR_END => irq::write(self, addr, data),

            timers::PADDR_START..=timers::PADDR_END => timers::write(self, addr, data),

            cdrom::PADDR_START..=cdrom::PADDR_END => cdrom::write(self, addr, data),

            sio::PADDR_START..=sio::PADDR_END => sio::write(self, addr, data),

            0x1F801000..=0x1F801023 => eprintln!("Unhandled write to memctl"),

            0x1F801060..=0x1F801063 => eprintln!("Unhandled write to ramsize"),

            0x1F801C00..=0x1F801E7F => eprintln!("Unhandled write to the SPU"),

            0xFFFE0130..=0xFFFE0133 => eprintln!("Unhandled write to cachectl"),

            0x1F000000..=0x1F0000FF => eprintln!("Unhandled write to the expansion1"),

            0x1F802000..=0x1F802041 => eprintln!("Unhandled write to expansion2"),

            _ => unimplemented!("write at {addr:#08X}"),
        };

        Ok(())
    }
}
