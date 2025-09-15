use crate::memory::utils::ByteAddressable;

pub struct Scratch {
    bytes: Box<[u8; 0x400]>,
}

impl Default for Scratch {
    fn default() -> Self {
        Self {
            bytes: Box::new([0; 0x400]),
        }
    }
}

impl Scratch {
    pub fn read<T: ByteAddressable>(&self, addr: u32) -> T {
        let addr = addr as usize;
        T::from_le_bytes(self.bytes[addr..addr + size_of::<T>()].try_into().unwrap())
    }

    pub fn write<T: ByteAddressable>(&mut self, addr: u32, val: T) {
        let addr = addr as usize;
        self.bytes[addr..addr + size_of::<T>()].copy_from_slice(val.to_le_bytes().as_ref());
    }
}
