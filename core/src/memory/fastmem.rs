use crate::memory::utils::ByteAddressable;
use std::error::Error;

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
