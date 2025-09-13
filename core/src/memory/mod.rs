mod bios;
mod map;
mod ram;
mod scratch;

use crate::Config;
use crate::cpu::utils::Exception;
use crate::dma::{
    Dma,
    utils::{Direction, Port, Step, Sync},
};
use crate::gpu::Gpu;
use bios::Bios;
use ram::Ram;
use scratch::Scratch;
use std::error::Error;

pub struct Bus {
    bios: Bios,
    pub gpu: Gpu,
    pub ram: Ram,
    scratch: Scratch,
    dma: Dma,
}

impl Bus {
    pub fn build(conf: &Config) -> Result<Self, Box<dyn Error>> {
        let bios = Bios::build(&conf.bios_path)?;
        let ram = Ram::default();
        let dma = Dma::default();
        let gpu = Gpu::default();
        let scratch = Scratch::default();

        Ok(Bus {
            gpu,
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

        if let Some(offs) = map::SCRATCH.contains(masked) {
            return self.scratch.read8(offs);
        }

        if let Some(offs) = map::EXPANSION1.contains(masked) {
            eprintln!("Unhandled read8 to the expansion1 {offs:x}");
            return 0;
        }

        if let Some(offs) = map::PERIPHERAL.contains(masked) {
            eprintln!("Unhandled read8 to the io port reg{offs:x}");
            return 0;
        }

        panic!("Unmapped read8 at {masked:#08X}");
    }

    pub fn read16(&self, addr: u32) -> Result<u16, Exception> {
        if addr & 1 != 0 {
            return Err(Exception::LoadAddressError(addr));
        }
        let masked = map::mask_region(addr);

        if let Some(offs) = map::RAM.contains(masked) {
            return Ok(self.ram.read16(offs));
        }

        if let Some(offs) = map::SCRATCH.contains(masked) {
            return Ok(self.scratch.read16(offs));
        }

        if let Some(offs) = map::SPU.contains(masked) {
            eprintln!("Unhandled read16 to the SPU reg{offs:x}");
            return Ok(0);
        }

        if let Some(offs) = map::PERIPHERAL.contains(masked) {
            eprintln!("Unhandled read16 to the io port reg{offs:x}");
            return Ok(0);
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            eprintln!("Unhandled read16 to the irqctl reg{offs:x}");
            return Ok(0);
        }

        panic!("Unmapped read16 at {masked:#08X}");
    }

    pub fn read32(&mut self, addr: u32) -> Result<u32, Exception> {
        if addr & 3 != 0 {
            return Err(Exception::LoadAddressError(addr));
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

        if let Some(offs) = map::TIMERS.contains(masked) {
            eprintln!("TIMER: {offs:x}");
            return Ok(0);
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            eprintln!("IRQCTL read: {offs:x}");
            return Ok(0);
        }

        if let Some(offs) = map::DMA.contains(masked) {
            return Ok(self.dma.get_reg(offs));
        }

        if let Some(offs) = map::GPU.contains(masked) {
            return match offs {
                4 => Ok(self.gpu.stat()),
                0 => Ok(self.gpu.read()),
                _ => panic!("Unknown GPU register read {offs:x}"),
            };
        }

        panic!("Unmapped read32 at {masked:#08X}");
    }

    pub fn write8(&mut self, addr: u32, data: u8) {
        let masked = map::mask_region(addr);

        if let Some(offs) = map::EXPANSION2.contains(masked) {
            return eprintln!("Unhandled write to expansion2 register{offs:x}");
        }

        if let Some(offs) = map::RAM.contains(masked) {
            return self.ram.write8(offs, data);
        }

        if let Some(offs) = map::SCRATCH.contains(masked) {
            return self.scratch.write8(offs, data);
        }

        panic!("Unmapped write8 at {addr:#08X}");
    }

    pub fn write16(&mut self, addr: u32, data: u16) -> Result<(), Exception> {
        if addr & 1 != 0 {
            return Err(Exception::StoreAddressError(addr));
        }
        let masked = map::mask_region(addr);

        if let Some(offs) = map::RAM.contains(masked) {
            self.ram.write16(offs, data);
            return Ok(());
        }

        if let Some(offs) = map::PERIPHERAL.contains(masked) {
            eprintln!("Unhandled write to io port reg{offs:x}");
            return Ok(());
        }

        if let Some(offs) = map::SCRATCH.contains(masked) {
            self.scratch.write16(offs, data);
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
            return Err(Exception::StoreAddressError(addr));
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

        if let Some(offs) = map::SCRATCH.contains(masked) {
            self.scratch.write32(offs, data);
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
            if let Some(port) = self.dma.set_reg(offs, data) {
                self.do_dma(port);
            }
            return Ok(());
        }

        if let Some(offs) = map::GPU.contains(masked) {
            match offs {
                0 => self.gpu.gp0(data),
                4 => self.gpu.gp1(data),
                _ => panic!("Unknown GPU register write {offs:x} <- {data:08x}"),
            }
            return Ok(());
        }

        if let Some(offs) = map::TIMERS.contains(masked) {
            eprintln!("TIMER: {offs:x} <- {data:08x}");
            return Ok(());
        }

        panic!("Unmapped write32 at {addr:#08X}");
    }

    pub fn do_dma(&mut self, port: Port) {
        match self.dma.channels[port as usize].ctl.sync() {
            Sync::LinkedList => self.do_dma_linked_list(port),
            _ => self.do_dma_block(port),
        }
    }

    pub fn do_dma_block(&mut self, port: Port) {
        let (step, dir, base, size) = {
            let channel = &mut self.dma.channels[port as usize];
            let step: i32 = match channel.ctl.step() {
                Step::Increment => 4,
                Step::Decrement => -4,
            };
            let size = channel.transfer_size().expect("Should not be none!");
            (step, channel.ctl.dir(), channel.base, size)
        };

        let mut addr = base;
        for s in (1..=size).rev() {
            let cur_addr = addr & 0x1FFFFC;
            match dir {
                Direction::ToRam => {
                    let src_word = match port {
                        Port::Otc => match s {
                            1 => 0xFFFFFF,
                            _ => addr.wrapping_sub(4) & 0x1FFFFF,
                        },
                        _ => panic!("Unhandled DMA source port"),
                    };
                    self.ram.write32(cur_addr, src_word);
                }
                Direction::FromRam => {
                    let src_word = self.ram.read32(cur_addr);
                    match port {
                        Port::Gpu => self.gpu.gp0(src_word),
                        _ => panic!("Unhandled DMA destination port"),
                    }
                }
            }
            addr = addr.wrapping_add_signed(step);
        }
        self.dma.channels[port as usize].done();
    }

    pub fn do_dma_linked_list(&mut self, port: Port) {
        let channel = &mut self.dma.channels[port as usize];
        if channel.ctl.dir() == Direction::ToRam {
            panic!("Invalid DMA direction for linked list mode.");
        }
        if port != Port::Gpu {
            panic!("Attempted linked list DMA on port {}", port as usize);
        }

        let mut addr = channel.base & 0x1FFFFC;
        loop {
            let header = self.ram.read32(addr);
            let size = header >> 24;

            for i in 0..size {
                let data = self.ram.read32(addr + 4 * (i + 1));
                self.gpu.gp0(data);
            }

            let next_addr = header & 0xFFFFFF;
            if next_addr & (1 << 23) != 0 {
                break;
            }
            addr = next_addr;
        }

        channel.done();
    }
}
