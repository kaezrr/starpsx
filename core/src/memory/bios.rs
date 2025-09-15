use crate::memory::utils::ByteAddressable;
use std::error::Error;

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
        let addr = addr as usize;
        T::from_le_bytes(self.bytes[addr..addr + size_of::<T>()].try_into().unwrap())
    }
}
