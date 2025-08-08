use super::*;

#[derive(Default)]
pub struct Cop0 {
    /// Cop0 reg12 : Status Register
    pub sr: u32,

    /// Cop0 reg13 : Exception Cause
    pub cause: u32,

    /// Cop0 reg14 : Exception Program Counter
    pub epc: u32,

    /// Cop0 reg8 : Bad Virtual Address
    pub baddr: u32,
}

impl Cpu {
    pub fn cop0(&mut self, instr: Instruction) -> Result<(), Exception> {
        match instr.rs() {
            0x00 => self.mfc0(instr),
            0x04 => self.mtc0(instr),
            0x10 => self.rfe(instr),
            _ => panic!("Unknown cop0 instruction {:#08X}", instr.0),
        }
    }
}
