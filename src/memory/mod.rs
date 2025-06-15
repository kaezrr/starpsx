use std::error::Error;

use crate::{Config, memory::bios::Bios};

mod bios;
mod map;

pub struct Bus {
    bios: Bios,
}

impl Bus {
    pub fn build(conf: Config) -> Result<Self, Box<dyn Error>> {
        let bios = Bios::build(&conf.bios_path)?;
        Ok(Bus { bios })
    }

    pub fn read8(&self, addr: u32) -> u8 {
        todo!()
    }

    pub fn read16(&self, addr: u32) -> u16 {
        todo!()
    }

    pub fn read32(&self, addr: u32) -> u32 {
        if let Some(addr) = map::BIOS.contains(addr) {
            self.bios.read32(addr)
        } else {
            todo!()
        }
    }

    pub fn write8(&mut self, addr: u32, data: u8) {
        todo!()
    }

    pub fn write16(&mut self, addr: u32, data: u16) {
        todo!()
    }

    pub fn write32(&mut self, addr: u32, data: u32) {
        todo!()
    }
}
