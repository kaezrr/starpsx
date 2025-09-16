use crate::memory::utils::ByteAddressable;

pub struct Ram {
    pub bytes: Box<[u8; 0x200000]>,
}

impl Default for Ram {
    fn default() -> Self {
        Self {
            bytes: Box::new([0; 0x200000]),
        }
    }
}

impl Ram {
    pub fn read<T: ByteAddressable>(&self, addr: u32) -> T {
        let addr = addr as usize;
        T::from_le_bytes(self.bytes[addr..addr + T::LEN].try_into().unwrap())
    }

    pub fn write<T: ByteAddressable>(&mut self, addr: u32, val: T) {
        let addr = addr as usize;
        self.bytes[addr..addr + T::LEN].copy_from_slice(val.to_le_bytes().as_ref());
    }
}
