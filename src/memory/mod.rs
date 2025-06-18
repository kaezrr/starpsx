mod bios;
mod map;
mod ram;

use crate::Config;
use crate::cpu::utils::Exception;
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

    pub fn read16(&self, addr: u32) -> Result<u16, Exception> {
        if addr & 1 != 0 {
            return Err(Exception::LoadAddressError);
        }
        let masked = map::mask_region(addr);

        if let Some(offs) = map::RAM.contains(masked) {
            return Ok(self.ram.read16(offs));
        }

        if let Some(offs) = map::SPU.contains(masked) {
            println!("Unhandled read to the SPU reg{:x}", offs);
            return Ok(0);
        }
        panic!("Unmapped read16 at {:#08X}", masked);
    }

    pub fn read32(&self, addr: u32) -> Result<u32, Exception> {
        if addr & 3 != 0 {
            return Err(Exception::LoadAddressError);
        }
        let masked = map::mask_region(addr);

        if let Some(offs) = map::BIOS.contains(masked) {
            return Ok(self.bios.read32(offs));
        }

        if let Some(offs) = map::RAM.contains(masked) {
            return Ok(self.ram.read32(offs));
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            println!("IRQCTL read: {:x}", offs);
            return Ok(0);
        }

        if let Some(offs) = map::DMA.contains(masked) {
            println!("DMA read: {:x}", offs);
            return Ok(0);
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

    pub fn write16(&mut self, addr: u32, data: u16) -> Result<(), Exception> {
        if addr & 1 != 0 {
            return Err(Exception::StoreAddressError);
        }
        let masked = map::mask_region(addr);

        if let Some(offs) = map::RAM.contains(masked) {
            self.ram.write16(offs, data);
            return Ok(());
        }

        if let Some(offs) = map::SPU.contains(masked) {
            println!("Unhandled write to the SPU reg{:x}", offs);
            return Ok(());
        }

        if let Some(offs) = map::TIMERS.contains(masked) {
            println!("TIMER: {:x} <- {:08x}", offs, data);
            return Ok(());
        }

        panic!("Unmapped write16 at {:#08X}", addr);
    }

    pub fn write32(&mut self, addr: u32, data: u32) -> Result<(), Exception> {
        if addr & 3 != 0 {
            return Err(Exception::StoreAddressError);
        }
        let masked = map::mask_region(addr);

        if let Some(offs) = map::RAM.contains(masked) {
            self.ram.write32(offs, data);
            return Ok(());
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
            return Ok(());
        }

        if map::RAMSIZE.contains(masked).is_some() {
            return Ok(());
        }

        if map::CACHECTL.contains(masked).is_some() {
            println!("Unhandled write to CACHECTL");
            return Ok(());
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            println!("IRQCTL: {:x} <- {:08x}", offs, data);
            return Ok(());
        }

        if let Some(offs) = map::DMA.contains(masked) {
            println!("DMA write: {:x}", offs);
            return Ok(());
        }

        panic!("Unmapped write32 at {:#08X}", addr);
    }
}
