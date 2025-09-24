mod flatmem;
mod handlers;
mod utils;

use crate::cpu::utils::Exception;
use crate::{Config, System, dma, gpu, irqctl, timer};
pub use flatmem::{
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
    scratch: Scratch,
    pub ram: Ram,
}

impl Bus {
    pub fn build(conf: &Config) -> Result<Self, Box<dyn Error>> {
        let bios = Bios::build(&conf.bios_path)?;
        let ram = Ram::default();
        let scratch = Scratch::default();

        Ok(Bus { bios, ram, scratch })
    }

    pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> Result<T, Exception> {
        if !addr.is_multiple_of(T::LEN as u32) {
            return Err(Exception::LoadAddressError(addr));
        }

        let addr = utils::mask_region(addr);

        let data = match addr {
            ram::PADDR_START..=ram::PADDR_END => system.bus.ram.read(addr),

            bios::PADDR_START..=bios::PADDR_END => system.bus.bios.read(addr),

            scratch::PADDR_START..=scratch::PADDR_END => system.bus.scratch.read(addr),

            gpu::PADDR_START..=gpu::PADDR_END => Bus::gpu_read_handler(system, addr),

            dma::PADDR_START..=dma::PADDR_END => Bus::dma_read_handler(system, addr),

            irqctl::PADDR_START..=irqctl::PADDR_END => Bus::irq_read_handler(system, addr),

            timer::PADDR_START..=timer::PADDR_END => Bus::timer_read_handler(system, addr),

            0x1F801000..=0x1F801023 => stubbed!("Unhandled read to memctl"),

            0x1F801060..=0x1F801063 => panic!("Unhandled read to ramsize"),

            0x1F801C00..=0x1F801E7F => stubbed!("Unhandled read to the SPU"),

            0xFFFE0130..=0xFFFE0133 => panic!("Unhandled read to cachectl"),

            0x1F000000..=0x1F0000FF => stubbed!("Unhandled read to the expansion1"),

            0x1F802000..=0x1F802041 => panic!("Unhandled read to expansion2"),

            0x1F801040..=0x1F80105F => stubbed!("Unhandled read to the io port"),

            _ => panic!("Unmapped read at {addr:#08X}"),
        };

        Ok(data)
    }

    pub fn write<T: ByteAddressable>(
        system: &mut System,
        addr: u32,
        data: T,
    ) -> Result<(), Exception> {
        if !addr.is_multiple_of(T::LEN as u32) {
            return Err(Exception::StoreAddressError(addr));
        }
        let addr = utils::mask_region(addr);

        match addr {
            ram::PADDR_START..=ram::PADDR_END => system.bus.ram.write(addr, data),

            scratch::PADDR_START..=scratch::PADDR_END => system.bus.scratch.write(addr, data),

            gpu::PADDR_START..=gpu::PADDR_END => Bus::gpu_write_handler(system, addr, data),

            dma::PADDR_START..=dma::PADDR_END => Bus::dma_write_handler(system, addr, data),

            irqctl::PADDR_START..=irqctl::PADDR_END => Bus::irq_write_handler(system, addr, data),

            timer::PADDR_START..=timer::PADDR_END => Bus::timer_write_handler(system, addr, data),

            0x1F801000..=0x1F801023 => eprintln!("Unhandled write to memctl"),

            0x1F801060..=0x1F801063 => eprintln!("Unhandled write to ramsize"),

            0x1F801C00..=0x1F801E7F => eprintln!("Unhandled write to the SPU"),

            0xFFFE0130..=0xFFFE0133 => eprintln!("Unhandled write to cachectl"),

            0x1F000000..=0x1F0000FF => eprintln!("Unhandled write to the expansion1"),

            0x1F802000..=0x1F802041 => eprintln!("Unhandled write to expansion2"),

            0x1F801040..=0x1F80105F => eprintln!("Unhandled write to the io port"),

            _ => panic!("Unmapped write at {addr:#08X}"),
        };

        Ok(())
    }
}
