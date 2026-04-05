use tracing::trace;

use crate::System;
use crate::cdrom;
use crate::cpu::utils::Exception;
use crate::dma;
use crate::gpu;
use crate::irq;
use crate::mdec;
use crate::sio;
use crate::spu;
use crate::timers;

pub mod bios {

    pub const PADDR_START: u32 = 0x1FC0_0000;
    pub const PADDR_END: u32 = 0x1FC8_0000;

    pub struct Bios {
        bytes: Box<[u8; 512 * 1024]>,
    }

    impl Bios {
        pub fn new(bytes: Vec<u8>) -> anyhow::Result<Self> {
            let box_bytes = bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid bios image"))?;

            Ok(Self { bytes: box_bytes })
        }

        pub fn read<const WIDTH: usize>(&self, addr: u32) -> u32 {
            let addr = (addr - PADDR_START) as usize;
            let mut buffer = [0u8; 4];

            buffer[..WIDTH].copy_from_slice(&self.bytes[addr..addr + WIDTH]);

            u32::from_le_bytes(buffer)
        }
    }
}

pub mod ram {
    pub const PADDR_START: u32 = 0x0000_0000;
    pub const PADDR_END: u32 = 0x0080_0000;

    pub struct Ram {
        pub bytes: Box<[u8; 0x20_0000]>,
    }

    impl Default for Ram {
        fn default() -> Self {
            Self {
                bytes: vec![0; 0x0020_0000].try_into().expect("ram alloc"),
            }
        }
    }

    impl Ram {
        pub fn read<const WIDTH: usize>(&self, addr: u32) -> u32 {
            let addr = (addr & 0x1FF_FFF) as usize;
            let mut buffer = [0u8; 4];

            buffer[..WIDTH].copy_from_slice(&self.bytes[addr..addr + WIDTH]);

            u32::from_le_bytes(buffer)
        }

        pub fn write<const WIDTH: usize>(&mut self, addr: u32, val: u32) {
            let addr = (addr & 0x1FF_FFF) as usize;
            let bytes = val.to_le_bytes(); // Convert u32 to [u8; 4]

            self.bytes[addr..addr + WIDTH].copy_from_slice(&bytes[..WIDTH]);
        }
    }
}

pub mod scratch {
    pub const PADDR_START: u32 = 0x1F80_0000;
    pub const PADDR_END: u32 = 0x1F80_0400;

    pub struct Scratch {
        bytes: Box<[u8; 0x400]>,
    }

    impl Default for Scratch {
        fn default() -> Self {
            Self {
                bytes: vec![0; 0x400].try_into().expect("scratch alloc"),
            }
        }
    }

    impl Scratch {
        pub fn read<const WIDTH: usize>(&self, addr: u32) -> u32 {
            let addr = (addr - PADDR_START) as usize;
            let mut buffer = [0u8; 4];

            buffer[..WIDTH].copy_from_slice(&self.bytes[addr..addr + WIDTH]);

            u32::from_le_bytes(buffer)
        }

        pub fn write<const WIDTH: usize>(&mut self, addr: u32, val: u32) {
            let addr = (addr - PADDR_START) as usize;
            let bytes = val.to_le_bytes(); // Convert u32 to [u8; 4]

            self.bytes[addr..addr + WIDTH].copy_from_slice(&bytes[..WIDTH]);
        }
    }
}

macro_rules! stubbed {
    ($region:expr, $at:expr) => {{
        trace!(target:"mem", region = $region, "stubbed read addr={:#08x}", $at);
        0xFF
    }};
}

const fn mask_region(addr: u32) -> u32 {
    addr & match addr >> 29 {
        0b100 => 0x7FFF_FFFF,                         // KSEG0
        0b101 => 0x1FFF_FFFF,                         // KSEG1
        0b000..=0b011 | 0b110 | 0b111 => 0xFFFF_FFFF, // KSEG2 | KUSEG
        _ => unreachable!(),
    }
}

impl System {
    #[must_use]
    pub fn fetch_instruction(&self, addr: u32) -> u32 {
        let addr = mask_region(addr);

        match addr {
            ram::PADDR_START..ram::PADDR_END => self.ram.read::<4>(addr),
            bios::PADDR_START..bios::PADDR_END => self.bios.read::<4>(addr),
            scratch::PADDR_START..scratch::PADDR_END => self.scratch.read::<4>(addr),
            _ => 0xFFFF_FFFF,
        }
    }

    /// # Errors
    /// Returns an error if the address is not properly aligned
    pub fn read<const WIDTH: usize>(&mut self, addr: u32) -> Result<u32, Exception> {
        if !addr.is_multiple_of(WIDTH as u32) {
            return Err(Exception::LoadAddressError(addr));
        }

        let addr = mask_region(addr);

        let data = match addr {
            ram::PADDR_START..ram::PADDR_END => self.ram.read::<WIDTH>(addr),

            bios::PADDR_START..bios::PADDR_END => self.bios.read::<WIDTH>(addr),

            scratch::PADDR_START..scratch::PADDR_END => self.scratch.read::<WIDTH>(addr),

            gpu::PADDR_START..gpu::PADDR_END => gpu::read::<WIDTH>(self, addr),

            dma::PADDR_START..dma::PADDR_END => dma::read::<WIDTH>(self, addr),

            irq::PADDR_START..irq::PADDR_END => irq::read::<WIDTH>(self, addr),

            timers::PADDR_START..timers::PADDR_END => timers::read::<WIDTH>(self, addr),

            cdrom::PADDR_START..cdrom::PADDR_END => cdrom::read::<WIDTH>(self, addr),

            sio::PADDR_START..sio::PADDR_END => sio::read::<WIDTH>(self, addr),

            spu::PADDR_START..spu::PADDR_END => spu::read::<WIDTH>(self, addr),

            mdec::PADDR_START..mdec::PADDR_END => mdec::read::<WIDTH>(self, addr),

            0x1F80_1000..0x1F80_1024 => stubbed!("memctl", addr),

            0x1F80_1060..0x1F80_1064 => 0xB88, // 2MB Ram Size

            0xFFFE_0130..0xFFFE_0134 => unimplemented!("read to cachectl"),

            0x1F00_0000..0x1F00_0100 => stubbed!("expansion1", addr),

            0x1F80_2000..0x1F80_2042 => unimplemented!("read to expansion2"),

            _ => unimplemented!("read at {addr:#08X}"),
        };

        // Mask the output so lbu/lhu get clean values
        let mask = match WIDTH {
            1 => 0xFF,
            2 => 0xFFFF,
            _ => 0xFFFF_FFFF,
        };

        Ok(data & mask)
    }

    /// # Errors
    /// Returns an error if the address is not properly aligned
    pub fn write<const WIDTH: usize>(&mut self, addr: u32, data: u32) -> Result<(), Exception> {
        if !addr.is_multiple_of(WIDTH as u32) {
            return Err(Exception::StoreAddressError(addr));
        }
        let addr = mask_region(addr);

        match addr {
            ram::PADDR_START..ram::PADDR_END => self.ram.write::<WIDTH>(addr, data),

            scratch::PADDR_START..scratch::PADDR_END => self.scratch.write::<WIDTH>(addr, data),

            gpu::PADDR_START..gpu::PADDR_END => gpu::write::<WIDTH>(self, addr, data),

            dma::PADDR_START..dma::PADDR_END => dma::write::<WIDTH>(self, addr, data),

            irq::PADDR_START..irq::PADDR_END => irq::write::<WIDTH>(self, addr, data),

            timers::PADDR_START..timers::PADDR_END => timers::write::<WIDTH>(self, addr, data),

            cdrom::PADDR_START..cdrom::PADDR_END => cdrom::write::<WIDTH>(self, addr, data),

            sio::PADDR_START..sio::PADDR_END => sio::write::<WIDTH>(self, addr, data),

            spu::PADDR_START..spu::PADDR_END => spu::write::<WIDTH>(self, addr, data),

            mdec::PADDR_START..mdec::PADDR_END => mdec::write::<WIDTH>(self, addr, data),

            0x1F80_1000..0x1F80_1024 => {
                trace!(target: "mem", region = "memctl", "stubbed write addr={:#08x}", addr);
            }
            0x1F80_1060..0x1F80_1064 => {
                trace!(target: "mem", region = "ramsize", "stubbed write addr={:#08x}", addr);
            }
            0xFFFE_0130..0xFFFE_0134 => {
                trace!(target: "mem", region = "cachectl", "stubbed write addr={:#08x}", addr);
            }
            0x1F00_0000..0x1F00_0100 => {
                trace!(target: "mem", region = "expansion1", "stubbed write addr={:#08x}", addr);
            }
            0x1F80_2000..0x1F80_2042 => {
                trace!(target: "mem", region = "expansion2", "stubbed write addr={:#08x}", addr);
            }

            _ => unimplemented!("write at {addr:#08X}"),
        }

        Ok(())
    }
}
