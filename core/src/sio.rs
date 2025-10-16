use crate::{System, mem::ByteAddressable};

pub const PADDR_START: u32 = 0x1F801040;
pub const PADDR_END: u32 = 0x1F80105F;

struct SerialInterface {}

pub fn read<T: ByteAddressable>(system: &System, addr: u32) -> T {
    todo!("read to the sio {addr:08x}");
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, val: T) {
    todo!("write to the sio {addr:08x} <- {val:08x}");
}
