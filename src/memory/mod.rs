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
        if addr & 3 != 0 {
            panic!("Unaligned read32 at {:#08X}", addr);
        }

        if let Some(addr) = map::BIOS.contains(addr) {
            self.bios.read32(addr)
        } else {
            panic!("Unmapped read32 at {:#08X}", addr);
        }
    }

    pub fn write8(&mut self, addr: u32, data: u8) {
        todo!()
    }

    pub fn write16(&mut self, addr: u32, data: u16) {
        todo!()
    }

    pub fn write32(&mut self, addr: u32, data: u32) {
        if addr & 3 != 0 {
            panic!("Unaligned write32 at {:#08X}", addr);
        }

        if let Some(addr) = map::MEMCTL.contains(addr) {
            match addr {
                0 => {
                    if data != 0x1F000000 {
                        panic!("Bad expansion 1 base address {:#08X}", data);
                    }
                }
                4 => {
                    if data != 0x1F802000 {
                        panic!("Bad expansion 2 base address {:#08X}", data);
                    }
                }
                _ => println!("Unhandled write to MEMCTRL"),
            }
        } else if map::RAMSIZE.contains(addr).is_some() {
            return;
        } else if map::CACHECTL.contains(addr).is_some() {
            println!("Unhandled write to CACHECTL");
            return;
        } else {
            panic!("Unmapped write32 at {:#08X}", addr);
        }
    }
}
