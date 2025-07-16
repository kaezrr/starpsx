mod bios;
mod map;
mod ram;
mod scratch;

use crate::Config;
use crate::cpu::utils::Exception;
use crate::dma::Dma;
use bios::Bios;
use ram::Ram;
use scratch::Scratch;
use std::error::Error;

pub struct Bus {
    bios: Bios,
    pub ram: Ram,
    scratch: Scratch,
    dma: Dma,
}

impl Bus {
    pub fn build(conf: &Config) -> Result<Self, Box<dyn Error>> {
        let bios = Bios::build(&conf.bios_path)?;
        let ram = Ram::new();
        let dma = Dma::new();
        let scratch = Scratch::new();

        Ok(Bus {
            bios,
            ram,
            scratch,
            dma,
        })
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
        panic!("Unmapped read8 at {masked:#08X}");
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
            eprintln!("Unhandled read to the SPU reg{offs:x}");
            return Ok(0);
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            eprintln!("IRQCTL: {offs:x} ");
            return Ok(0);
        }

        panic!("Unmapped read16 at {masked:#08X}");
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

        if let Some(offs) = map::SCRATCH.contains(masked) {
            return Ok(self.scratch.read32(offs));
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            eprintln!("IRQCTL read: {offs:x}");
            return Ok(0);
        }

        if let Some(offs) = map::DMA.contains(masked) {
            eprintln!("DMA read: {offs:x}");
            return Ok(self.dma.get_reg(offs));
        }

        if let Some(offs) = map::GPU.contains(masked) {
            eprintln!("GPU read: {offs:x}");
            return match offs {
                // GPU STAT ready for DMA
                4 => Ok(0x5E800000),
                _ => Ok(0),
            };
        }

        panic!("Unmapped read32 at {masked:#08X}");
    }

    pub fn write8(&mut self, addr: u32, data: u8) {
        let masked = map::mask_region(addr);

        if let Some(offs) = map::EXPANSION2.contains(masked) {
            if addr == 0x1F802041 {
                println!("POST: {data}");
            }
            return eprintln!("Unhandled write to expansion2 register{offs:x}");
        }

        if let Some(offs) = map::RAM.contains(masked) {
            return self.ram.write8(offs, data);
        }

        panic!("Unmapped write8 at {addr:#08X}");
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
            eprintln!("Unhandled write to the SPU reg{offs:x}");
            return Ok(());
        }

        if let Some(offs) = map::TIMERS.contains(masked) {
            eprintln!("TIMER: {offs:x} <- {data:08x}");
            return Ok(());
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            eprintln!("IRQCTL: {offs:x} <- {data:08x}");
            return Ok(());
        }

        panic!("Unmapped write16 at {addr:#08X}");
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
                        panic!("Bad expansion 1 base address {data:#08X}");
                    }
                }
                4 => {
                    if data != 0x1F802000 {
                        panic!("Bad expansion 2 base address {data:#08X}");
                    }
                }
                _ => eprintln!("Unhandled write to MEMCTRL"),
            }
            return Ok(());
        }

        if map::RAMSIZE.contains(masked).is_some() {
            return Ok(());
        }

        if map::CACHECTL.contains(masked).is_some() {
            eprintln!("Unhandled write to CACHECTL");
            return Ok(());
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            eprintln!("IRQCTL: {offs:x} <- {data:08x}");
            return Ok(());
        }

        if let Some(offs) = map::DMA.contains(masked) {
            eprintln!("DMA write: {offs:x}");
            self.dma.set_reg(offs, data);
            return Ok(());
        }

        if let Some(offs) = map::GPU.contains(masked) {
            eprintln!("GPU {offs:x} write: {data:x}");
            return Ok(());
        }

        if let Some(offs) = map::TIMERS.contains(masked) {
            eprintln!("TIMER: {offs:x} <- {data:08x}");
            return Ok(());
        }

        panic!("Unmapped write32 at {addr:#08X}");
    }
}
