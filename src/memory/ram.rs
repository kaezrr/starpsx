pub struct Ram {
    pub bytes: Vec<u8>,
}

impl Ram {
    pub fn new() -> Self {
        let bytes = vec![0; 0x200000];
        Ram { bytes }
    }

    pub fn read8(&self, addr: u32) -> u8 {
        self.bytes[addr as usize]
    }

    pub fn read16(&self, addr: u32) -> u16 {
        u16::from_le_bytes(*self.bytes[(addr as usize)..].first_chunk().unwrap())
    }

    pub fn read32(&self, addr: u32) -> u32 {
        u32::from_le_bytes(*self.bytes[(addr as usize)..].first_chunk().unwrap())
    }

    pub fn write8(&mut self, addr: u32, val: u8) {
        self.bytes[addr as usize] = val
    }

    pub fn write16(&mut self, addr: u32, val: u16) {
        *self.bytes[(addr as usize)..].first_chunk_mut().unwrap() = val.to_le_bytes();
    }

    pub fn write32(&mut self, addr: u32, val: u32) {
        *self.bytes[(addr as usize)..].first_chunk_mut().unwrap() = val.to_le_bytes();
    }
}
