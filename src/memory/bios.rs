use std::{error::Error, fs};

pub struct Bios {
    bytes: Vec<u8>,
}

impl Bios {
    pub fn build(bios_path: &String) -> Result<Self, Box<dyn Error>> {
        let bytes = fs::read(bios_path)?;
        Ok(Bios { bytes })
    }

    pub fn read32(&self, addr: u32) -> u32 {
        u32::from_le_bytes(*self.bytes[(addr as usize)..].first_chunk().unwrap())
    }
}
