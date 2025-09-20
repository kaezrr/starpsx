bitfield::bitfield! {
    #[derive(Copy, Clone)]
    pub struct Instruction(u32);
    pub u8, pri, _ : 31, 26;
    pub u8, sec, _ : 5, 0;
    rs_raw, _ : 25, 21;
    rt_raw, _ : 20, 16;
    rd_raw, _ : 15, 11;
    pub imm5, _ : 10, 6;
    pub imm16, _ : 15, 0;
    i16, imm16_se_raw, _ : 15, 0;
    pub imm26, _ : 25, 0;
}

impl Instruction {
    pub fn rs(&self) -> usize {
        self.rs_raw() as usize
    }

    pub fn rt(&self) -> usize {
        self.rt_raw() as usize
    }

    pub fn rd(&self) -> usize {
        self.rd_raw() as usize
    }

    pub fn imm16_se(&self) -> u32 {
        self.imm16_se_raw() as u32
    }
}

#[derive(Debug)]
pub enum Exception {
    ExternalInterrupt,
    LoadAddressError(u32),
    StoreAddressError(u32),
    Syscall,
    Break,
    IllegalInstruction,
    CoprocessorError,
    Overflow,
}

impl Exception {
    pub fn code(&self) -> u32 {
        match self {
            Exception::ExternalInterrupt => 0x0,
            Exception::LoadAddressError(_) => 0x4,
            Exception::StoreAddressError(_) => 0x5,
            Exception::Syscall => 0x8,
            Exception::Break => 0x9,
            Exception::IllegalInstruction => 0xA,
            Exception::CoprocessorError => 0xB,
            Exception::Overflow => 0xC,
        }
    }
}
