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
    pub fn read32(&self, addr: u32) -> u32 {
        u32::from_le_bytes(*self.bytes[(addr as usize)..].first_chunk().unwrap())
    }
}
