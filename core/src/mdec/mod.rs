use crate::{System, mem::ByteAddressable};

pub const PADDR_START: u32 = 0x1F801820;
pub const PADDR_END: u32 = 0x1F801827;

pub fn read<T: ByteAddressable>(system: &System, addr: u32) -> T {
    todo!("MDEC read {addr:x}");
}

pub fn write<T: ByteAddressable>(system: &System, addr: u32, data: T) {
    todo!("MDEC write {addr:x}");
}
