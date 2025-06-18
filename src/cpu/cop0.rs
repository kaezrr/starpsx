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
    pub fn cop0(&mut self, instr: Opcode) {
        match instr.rs() {
            0x00 => self.mfc0(instr),
            0x04 => self.mtc0(instr),
            _ => panic!("Unknown cop0 instruction {:#08X}", instr.0),
        }
    }

    /// Move to cop0 register
    pub fn mtc0(&mut self, instr: Opcode) {
        let cpu_r = instr.rt();
        let cop_r = instr.rd();

        let data = self.regs[cpu_r];

        match cop_r {
            3 | 5 | 6 | 7 | 9 | 11 | 13 => {
                if data != 0 {
                    panic!("Unhandled write to cop0r{}", cop_r);
                }
            }
            12 => self.cop0.sr = data,
            _ => panic!("Unhandled cop0 register {cop_r}"),
        }
    }

    /// Move from cop0 register
    pub fn mfc0(&mut self, instr: Opcode) {
        let cpu_r = instr.rt();
        let cop_r = instr.rd();

        let data = match cop_r {
            12 => self.cop0.sr,
            13 => self.cop0.cause,
            14 => self.cop0.epc,
            _ => panic!("Unhandled cop0 register {cop_r}"),
        };

        self.load = (cpu_r, data);
    }
}
