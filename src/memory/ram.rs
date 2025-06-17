pub struct Ram {
    bytes: Vec<u8>,
}

impl Ram {
    pub fn new() -> Self {
        let bytes = vec![0; 2048 * 1024];
        Ram { bytes }
    }

    pub fn read32(&self, addr: u32) -> u32 {
        u32::from_le_bytes(*self.bytes[(addr as usize)..].first_chunk().unwrap())
    }

    pub fn write32(&mut self, addr: u32, val: u32) {
        *self.bytes[(addr as usize)..].first_chunk_mut().unwrap() = val.to_le_bytes();
    }
}
