mod fastmem;
mod handlers;
mod utils;

use crate::Config;
use crate::cpu::utils::Exception;
use crate::dma::{self, Dma};
use crate::gpu::{self, Gpu};
pub use fastmem::{
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

            scratch::PADDR_START..=scratch::PADDR_END => self.scratch.read(addr),

            gpu::PADDR_START..=gpu::PADDR_END => self.gpu_read_handler(addr),

            dma::PADDR_START..=dma::PADDR_END => self.dma_read_handler(addr),

            0x1F801000..=0x1F801023 => panic!("Unhandled read to memctl"),

            0x1F801060..=0x1F801063 => panic!("Unhandled read to ramsize"),

            0x1F801C00..=0x1F801E7F => stubbed!("Unhandled read to the SPU reg"),

            0xFFFE0130..=0xFFFE0133 => panic!("Unhandled read to cachectl"),

            0x1F802000..=0x1F802041 => panic!("Unhandled read to expansion2"),

            0x1F000000..=0x1F0000FF => stubbed!("Unhandled read to the expansion1"),

            0x1F801070..=0x1F801077 => stubbed!("Unhandled read to the irqctl reg{addr:08x}"),

            0x1F801100..=0x1F80112F => stubbed!("Unhandled read to the timers"),

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

            scratch::PADDR_START..=scratch::PADDR_END => self.scratch.write(addr, data),

            gpu::PADDR_START..=gpu::PADDR_END => self.gpu_write_handler(addr, data),

            dma::PADDR_START..=dma::PADDR_END => self.dma_write_handler(addr, data),

            0x1F801000..=0x1F801023 => eprintln!("Unhandled write to memctl"),

            0x1F801060..=0x1F801063 => eprintln!("Unhandled write to ramsize"),

            0x1F801C00..=0x1F801E7F => eprintln!("Unhandled write to the SPU"),

            0xFFFE0130..=0xFFFE0133 => eprintln!("Unhandled write to cachectl"),

            0x1F802000..=0x1F802041 => eprintln!("Unhandled write to expansion2"),

            0x1F000000..=0x1F0000FF => eprintln!("Unhandled write to the expansion1"),

            0x1F801070..=0x1F801077 => eprintln!("Unhandled write to the irqctl reg{addr:08x}"),

            0x1F801100..=0x1F80112F => eprintln!("Unhandled write to the irqctl"),

            0x1F801040..=0x1F80105F => eprintln!("Unhandled write to the io port"),

            _ => panic!("Unmapped write at {addr:#08X}"),
        };

        Ok(())
    }
}
