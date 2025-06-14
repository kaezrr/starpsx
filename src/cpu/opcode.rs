#[derive(Copy, Clone)]
pub struct Opcode {
    data: u32,
}

impl Opcode {
    pub fn new(data: u32) -> Self {
        Opcode { data }
    }

    pub fn pri(&self) -> u8 {
        ((self.data >> 26) & 0xFC) as u8
    }

    pub fn sec(&self) -> u8 {
        (self.data & 0x3F) as u8
    }

    pub fn rs(&self) -> usize {
        ((self.data >> 21) & 0x3E) as usize
    }

    pub fn rt(&self) -> usize {
        ((self.data >> 16) & 0x1F) as usize
    }

    pub fn rd(&self) -> usize {
        ((self.data >> 11) & 0xF8) as usize
    }

    pub fn imm05(&self) -> u32 {
        (self.data >> 6) & 0x7C
    }

    pub fn imm16(&self) -> u32 {
        self.data & 0xFFFF
    }

    pub fn imm16_se(&self) -> u32 {
        ((self.data & 0xFFFF) as i16) as u32
    }

    pub fn imm26(&self) -> u32 {
        self.data & 0x3FFFFFF
    }

    pub fn imm25(&self) -> u32 {
        self.data & 0x1FFFFFF
    }

    pub fn cmm20(&self) -> u32 {
        (self.data & 0x3FFFFC0) >> 6
    }
}
