pub struct Scratch {
    bytes: Vec<u8>,
}

impl Scratch {
    pub fn new() -> Self {
        Scratch {
            bytes: vec![0; 0x400],
        }
    }

    pub fn read32(&self, addr: u32) -> u32 {
        u32::from_le_bytes(*self.bytes[(addr as usize)..].first_chunk().unwrap())
    }
}
