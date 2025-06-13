pub struct Opcode {
    data: u32,
}

impl Opcode {
    pub fn new() -> Self {
        Opcode { data: 0 }
    }

    pub fn set(&mut self, data: u32) {
        self.data = data;
    }

    pub fn pri(&self) -> u32 {
        let mask = 0b111111_00000_00000_00000_00000_000000;
        (self.data & mask) >> 26
    }

    pub fn sec(&self) -> u32 {
        let mask = 0b000000_00000_00000_00000_00000_111111;
        (self.data & mask) >> 0
    }

    pub fn rs(&self) -> u32 {
        let mask = 0b000000_11111_00000_00000_00000_000000;
        (self.data & mask) >> 21
    }

    pub fn rt(&self) -> u32 {
        let mask = 0b000000_00000_11111_00000_00000_000000;
        (self.data & mask) >> 16
    }

    pub fn rd(&self) -> u32 {
        let mask = 0b000000_00000_00000_11111_00000_000000;
        (self.data & mask) >> 11
    }

    pub fn imm05(&self) -> u32 {
        let mask = 0b000000_00000_00000_00000_11111_000000;
        (self.data & mask) >> 6
    }

    pub fn imm16(&self) -> u32 {
        let mask = 0b000000_00000_00000_11111_11111_111111;
        (self.data & mask) >> 0
    }

    pub fn imm26(&self) -> u32 {
        let mask = 0b000000_11111_11111_11111_11111_111111;
        (self.data & mask) >> 0
    }

    pub fn imm25(&self) -> u32 {
        let mask = 0b000000_01111_11111_11111_11111_111111;
        (self.data & mask) >> 0
    }

    pub fn cmm20(&self) -> u32 {
        let mask = 0b000000_11111_11111_11111_11111_000000;
        (self.data & mask) >> 6
    }
}
