mod fastmem;
mod handlers;
mod utils;

use crate::Config;
use crate::cpu::utils::Exception;
use crate::dma::{
    self, Dma,
    utils::{Direction, Port, Step, Sync},
};
use crate::gpu::{self, Gpu};
use fastmem::{
    bios::{self, Bios},
    ram::{self, Ram},
    scratch::{self, Scratch},
};
use std::error::Error;
use utils::ByteAddressable;

macro_rules! stubbed {
    ($region:expr) => {{
        eprintln!($region);
        T::from_u32(0)
    }};
}

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

        let addr = utils::mask_region(addr);

        let data = match addr {
            ram::PADDR_START..=ram::PADDR_END => self.ram.read(addr),

            bios::PADDR_START..=bios::PADDR_END => self.bios.read(addr),

            0x1F801000..=0x1F801023 => panic!("Unhandled read to memctl"),

            0x1F801060..=0x1F801063 => panic!("Unhandled read to ramsize"),

            0x1F801C00..=0x1F801E7F => stubbed!("Unhandled read to the SPU reg"),

            0xFFFE0130..=0xFFFE0133 => panic!("Unhandled read to cachectl"),

            0x1F802000..=0x1F802041 => panic!("Unhandled read to expansion2"),

            0x1F000000..=0x1F0000FF => stubbed!("Unhandled read to the expansion1"),

            0x1F801070..=0x1F801077 => stubbed!("Unhandled read to the irqctl reg{addr:08x}"),

            0x1F801100..=0x1F80112F => stubbed!("Unhandled read to the timers"),

            dma::PADDR_START..=dma::PADDR_END => self.dma_read_handler(addr),

            gpu::PADDR_START..=gpu::PADDR_END => self.gpu_read_handler(addr),

            scratch::PADDR_START..=scratch::PADDR_END => self.scratch.read(addr),

            0x1F801040..=0x1F80105F => stubbed!("Unhandled read to the io port"),

            _ => panic!("Unmapped read at {addr:#08X}"),
        };

        Ok(data)
    }

    pub fn write<T: ByteAddressable>(&mut self, addr: u32, data: T) -> Result<(), Exception> {
        if !addr.is_multiple_of(T::LEN as u32) {
            return Err(Exception::StoreAddressError(addr));
        }
        let addr = utils::mask_region(addr);

        match addr {
            ram::PADDR_START..=ram::PADDR_END => self.ram.write(addr, data),

            0x1F801000..=0x1F801023 => eprintln!("Unhandled write to memctl"),

            0x1F801060..=0x1F801063 => eprintln!("Unhandled write to ramsize"),

            0x1F801C00..=0x1F801E7F => eprintln!("Unhandled write to the SPU"),

            0xFFFE0130..=0xFFFE0133 => eprintln!("Unhandled write to cachectl"),

            0x1F802000..=0x1F802041 => eprintln!("Unhandled write to expansion2"),

            0x1F000000..=0x1F0000FF => eprintln!("Unhandled write to the expansion1"),

            0x1F801070..=0x1F801077 => eprintln!("Unhandled write to the irqctl reg{addr:08x}"),

            0x1F801100..=0x1F80112F => eprintln!("Unhandled write to the irqctl"),

            dma::PADDR_START..=dma::PADDR_END => self.dma_write_handler(addr, data),

            gpu::PADDR_START..=gpu::PADDR_END => self.gpu_write_handler(addr, data),

            scratch::PADDR_START..=scratch::PADDR_END => self.scratch.write(addr, data),

            0x1F801040..=0x1F80105F => eprintln!("Unhandled write to the io port"),

            _ => panic!("Unmapped write at {addr:#08X}"),
        };

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
