mod bios;
mod handlers;
mod ram;
mod scratch;
mod utils;

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
use utils::{ByteAddressable, map};

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

    pub fn read<T: ByteAddressable>(&mut self, addr: u32) -> Result<T, Exception> {
        if !addr.is_multiple_of(T::LEN as u32) {
            return Err(Exception::LoadAddressError(addr));
        }

        let masked = utils::mask_region(addr);

        if let Some(offs) = map::BIOS.contains(masked) {
            return Ok(self.bios.read::<T>(offs));
        }

        if let Some(offs) = map::RAM.contains(masked) {
            return Ok(self.ram.read::<T>(offs));
        }

        if let Some(offs) = map::SCRATCH.contains(masked) {
            return Ok(self.scratch.read::<T>(offs));
        }

        if let Some(offs) = map::EXPANSION1.contains(masked) {
            eprintln!("Unhandled read to the expansion1 {offs:x}");
            return Ok(T::zeroed());
        }

        if let Some(offs) = map::SPU.contains(masked) {
            eprintln!("Unhandled read to the SPU reg{offs:x}");
            return Ok(T::zeroed());
        }

        if let Some(offs) = map::PERIPHERAL.contains(masked) {
            eprintln!("Unhandled read to the io port reg{offs:x}");
            return Ok(T::zeroed());
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            eprintln!("Unhandled read to the irqctl reg{offs:x}");
            return Ok(T::zeroed());
        }

        if let Some(offs) = map::TIMERS.contains(masked) {
            eprintln!("TIMER: {offs:x}");
            return Ok(T::zeroed());
        }

        if let Some(offs) = map::DMA.contains(masked) {
            return Ok(self.dma_read_handler::<T>(offs));
        }

        if let Some(offs) = map::GPU.contains(masked) {
            return Ok(self.gpu_read_handler::<T>(offs));
        }

        panic!("Unmapped read8 at {masked:#08X}");
    }

    pub fn write<T: ByteAddressable>(&mut self, addr: u32, data: T) -> Result<(), Exception> {
        if !addr.is_multiple_of(T::LEN as u32) {
            return Err(Exception::StoreAddressError(addr));
        }
        let masked = utils::mask_region(addr);

        if let Some(offs) = map::RAM.contains(masked) {
            self.ram.write::<T>(offs, data);
            return Ok(());
        }

        if let Some(offs) = map::SCRATCH.contains(masked) {
            self.scratch.write::<T>(offs, data);
            return Ok(());
        }

        if let Some(offs) = map::EXPANSION2.contains(masked) {
            eprintln!("Unhandled write to expansion2 register{offs:x}");
            return Ok(());
        }

        if let Some(offs) = map::PERIPHERAL.contains(masked) {
            eprintln!("Unhandled write to io port reg{offs:x}");
            return Ok(());
        }

        if let Some(offs) = map::SPU.contains(masked) {
            eprintln!("Unhandled write to the SPU reg{offs:x}");
            return Ok(());
        }

        if let Some(offs) = map::TIMERS.contains(masked) {
            eprintln!("Unhandled write to the TIMERS reg{offs:x}");
            return Ok(());
        }

        if let Some(offs) = map::IRQCTL.contains(masked) {
            eprintln!("Unhandled write to the IRQCTL reg{offs:x}");
            return Ok(());
        }

        if let Some(offs) = map::MEMCTL.contains(masked) {
            self.memctl_write_handler(offs, data);
            return Ok(());
        }

        if map::RAMSIZE.contains(masked).is_some() {
            return Ok(());
        }

        if map::CACHECTL.contains(masked).is_some() {
            eprintln!("Unhandled write to CACHECTL");
            return Ok(());
        }

        if let Some(offs) = map::DMA.contains(masked) {
            self.dma_write_handler(offs, data);
            return Ok(());
        }

        if let Some(offs) = map::GPU.contains(masked) {
            self.gpu_write_handler(offs, data);
            return Ok(());
        }

        Ok(())
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
                    self.ram.write::<u32>(cur_addr, src_word);
                }
                Direction::FromRam => {
                    let src_word = self.ram.read::<u32>(cur_addr);
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
            let header = self.ram.read::<u32>(addr);
            let size = header >> 24;

            for i in 0..size {
                let data = self.ram.read::<u32>(addr + 4 * (i + 1));
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
