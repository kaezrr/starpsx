mod bios;
mod map;
mod ram;

use crate::Config;
use bios::Bios;
use ram::Ram;
use std::error::Error;

pub struct Bus {
    bios: Bios,
    ram: Ram,
}

impl Bus {
    pub fn build(conf: Config) -> Result<Self, Box<dyn Error>> {
        let bios = Bios::build(&conf.bios_path)?;
        let ram = Ram::new();

        Ok(Bus { bios, ram })
    }

    pub fn read8(&self, addr: u32) -> u8 {
        let masked = map::mask_region(addr);

        if let Some(offs) = map::BIOS.contains(masked) {
            return self.bios.read8(offs);
        }

        if let Some(offs) = map::RAM.contains(masked) {
            return self.ram.read8(offs);
        }

        if map::EXPANSION1.contains(masked).is_some() {
            return 0xFF;
        }
        panic!("Unmapped read8 at {:#08X}", masked);
    }

    pub fn read16(&self, addr: u32) -> u16 {
        todo!()
    }

    pub fn read32(&self, addr: u32) -> u32 {
        if addr & 3 != 0 {
            panic!("Unaligned read32 at {:#08X}", addr);
        }
        let masked = map::mask_region(addr);

        if let Some(offs) = map::BIOS.contains(masked) {
            return self.bios.read32(offs);
        }

        if let Some(offs) = map::RAM.contains(masked) {
            return self.ram.read32(offs);
        }

        panic!("Unmapped read32 at {:#08X}", masked);
    }

    pub fn write8(&mut self, addr: u32, data: u8) {
        let masked = map::mask_region(addr);

        if let Some(offs) = map::EXPANSION2.contains(masked) {
            return println!("Unhandled write to expansion2 register{:x}", offs);
        }

        if let Some(offs) = map::RAM.contains(masked) {
            return self.ram.write8(offs, data);
        }

        panic!("Unmapped write8 at {:#08X}", addr);
    }

    pub fn write16(&mut self, addr: u32, data: u16) {
        if addr & 1 != 0 {
            panic!("Unaligned write16 at {:#08X}", addr);
        }
        let masked = map::mask_region(addr);

        if let Some(offs) = map::SPU.contains(masked) {
            return println!("Unhandled write to the SPU reg{:x}", offs);
        }

        panic!("Unmapped write16 at {:#08X}", addr);
    }

    pub fn write32(&mut self, addr: u32, data: u32) {
        if addr & 3 != 0 {
            panic!("Unaligned write32 at {:#08X}", addr);
        }
        let masked = map::mask_region(addr);

        if let Some(offs) = map::RAM.contains(masked) {
            return self.ram.write32(offs, data);
        }

        if let Some(offs) = map::MEMCTL.contains(masked) {
            match offs {
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
            return;
        }

        if map::RAMSIZE.contains(masked).is_some() {
            return;
        }

        if map::CACHECTL.contains(masked).is_some() {
            return println!("Unhandled write to CACHECTL");
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            return println!("IRQCTL: {:x} <- {:08x}", offs, data);
        }

        panic!("Unmapped write32 at {:#08X}", addr);
    }
}
