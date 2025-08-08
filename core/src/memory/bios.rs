use std::{error::Error, fs};

pub struct Bios {
    bytes: Box<[u8; 512 * 1024]>,
}

impl Bios {
    pub fn build(bios_path: &String) -> Result<Self, Box<dyn Error>> {
        let bytes = match fs::read(bios_path)?.try_into() {
            Ok(data) => data,
            Err(_) => return Err("invalid bios file.".into()),
        };

        Ok(Bios {
            bytes: Box::new(bytes),
        })
    }

    pub fn read8(&self, addr: u32) -> u8 {
        self.bytes[addr as usize]
    }

    pub fn read32(&self, addr: u32) -> u32 {
        u32::from_le_bytes(*self.bytes[(addr as usize)..].first_chunk().unwrap())
    }
}
