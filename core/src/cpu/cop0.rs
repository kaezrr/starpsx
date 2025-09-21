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

    /// Move to cop0 register
    pub fn mtc0(&mut self, instr: Instruction) -> Result<(), Exception> {
        let cpu_r = instr.rt();
        let cop_r = instr.rd();

        let data = self.regs[cpu_r];

        match cop_r {
            3 | 5 | 6 | 7 | 9 | 11 | 15 => (),
            8 => self.cop0.baddr = data,
            12 => self.cop0.sr = data,
            // Only bits 8 and 9 are writable
            13 => self.cop0.cause = (self.cop0.cause & !0x300) & (data & 0x300),
            _ => panic!("Unhandled cop0r{cop_r} write <- {data:x}"),
        }
        Ok(())
    }

    /// Move from cop0 register
    pub fn mfc0(&mut self, instr: Instruction) -> Result<(), Exception> {
        let cpu_r = instr.rt();
        let cop_r = instr.rd();

        let data = match cop_r {
            3 | 5 | 6 | 7 | 9 | 11 | 15 => 0,
            8 => self.cop0.baddr,
            12 => self.cop0.sr,
            13 => self.cop0.cause,
            14 => self.cop0.epc,
            _ => panic!("Unhandled cop0 register read {cop_r}"),
        };

        self.take_delayed_load(cpu_r, data);
        Ok(())
    }

    /// Return from exception
    pub fn rfe(&mut self, instr: Instruction) -> Result<(), Exception> {
        if instr.sec() != 0x10 {
            panic!("Invalid cop0 instruction: {}", instr.0);
        }

        let mode = self.cop0.sr & 0x3F;
        self.cop0.sr = (self.cop0.sr & !0xF) | (mode >> 2);
        Ok(())
    }
}
