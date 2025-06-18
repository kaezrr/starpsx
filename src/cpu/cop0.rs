use super::*;

pub struct Cop0 {
    /// Cop0 reg12 : Status Register
    pub sr: u32,

    /// Cop0 reg13 : Exception Cause
    pub cause: u32,

    /// Cop0 reg14 : Exception Program Counter
    pub epc: u32,
}

impl Cop0 {
    pub fn new() -> Self {
        Cop0 {
            sr: 0,
            epc: 0,
            cause: 0,
        }
    }
}

impl Cpu {
    pub fn cop0(&mut self, instr: Opcode) -> Result<(), Exception> {
        match instr.rs() {
            0x00 => self.mfc0(instr),
            0x04 => self.mtc0(instr),
            0x10 => self.rfe(instr),
            _ => panic!("Unknown cop0 instruction {:#08X}", instr.0),
        }
    }
}
