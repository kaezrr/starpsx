#[derive(Copy, Clone)]
pub struct Opcode(pub u32);

impl Opcode {
    pub fn pri(&self) -> u8 {
        (self.0 >> 26) as u8
    }

    pub fn sec(&self) -> u8 {
        (self.0 & 0x3F) as u8
    }

    pub fn rs(&self) -> usize {
        ((self.0 >> 21) & 0x1F) as usize
    }

    pub fn rt(&self) -> usize {
        ((self.0 >> 16) & 0x1F) as usize
    }

    pub fn rd(&self) -> usize {
        ((self.0 >> 11) & 0x1F) as usize
    }

    pub fn imm5(&self) -> u32 {
        (self.0 >> 6) & 0x1F
    }

    pub fn imm16(&self) -> u32 {
        self.0 & 0xFFFF
    }

    pub fn imm16_se(&self) -> u32 {
        ((self.0 & 0xFFFF) as i16) as u32
    }

    pub fn imm26(&self) -> u32 {
        self.0 & 0x3FFFFFF
    }

    pub fn imm25(&self) -> u32 {
        self.0 & 0x1FFFFFF
    }

    pub fn cmm20(&self) -> u32 {
        (self.0 >> 6) & 0xFFFFF
    }
}

pub enum Exception {
    Syscall = 0x8,
}
