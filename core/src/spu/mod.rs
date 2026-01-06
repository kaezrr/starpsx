use tracing::trace;

use crate::{System, mem::ByteAddressable};

pub const PADDR_START: u32 = 0x1F801C00;
pub const PADDR_END: u32 = 0x1F801E7F;

// This is a stubbed component right now, just returns whatever is written
pub struct Spu {
    stubbed_registers: [u8; 640],
}

impl Default for Spu {
    fn default() -> Self {
        Self {
            stubbed_registers: [0; 640],
        }
    }
}

pub fn read<T: ByteAddressable>(system: &System, addr: u32) -> T {
    trace!(target: "mem", "stubbed spu read addr={addr:08x}");
    let addr = (addr - PADDR_START) as usize;
    T::from_le_bytes(
        system.spu.stubbed_registers[addr..addr + T::LEN]
            .try_into()
            .unwrap(),
    )
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, val: T) {
    trace!(target: "mem", "stubbed spu write addr={addr:08x}, data={:08x}", val.to_u32());
    let addr = (addr - PADDR_START) as usize;
    system.spu.stubbed_registers[addr..addr + T::LEN].copy_from_slice(val.to_le_bytes().as_ref());
}
