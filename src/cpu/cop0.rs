use super::*;

pub struct Cop0 {
    /// Cop0 reg12 : Status Register
    pub sr: u32,
}

impl Cop0 {
    pub fn new() -> Self {
        Cop0 { sr: 0 }
    }
}

impl Cpu {
    pub fn cop0(&mut self, instr: Opcode) {
        match instr.rs() {
            0x00 => self.mfc0(instr),
            0x04 => self.mtc0(instr),
            _ => panic!("Unknown cop0 instruction {:#08X}", instr.0),
        }
    }
}
