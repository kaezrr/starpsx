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

impl Cop0 {
    pub fn cop0(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        match instr.rs() {
            0x00 => Cop0::mfc0(system, instr),
            0x04 => Cop0::mtc0(system, instr),
            0x10 => Cop0::rfe(system, instr),
            _ => panic!("Unknown cop0 instruction {:#08X}", instr.0),
        }
    }

    /// Move to cop0 register
    pub fn mtc0(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let cpu_r = instr.rt();
        let cop_r = instr.rd();

        let data = system.cpu.regs[cpu_r];

        match cop_r {
            3 | 5 | 6 | 7 | 9 | 11 | 15 => (),
            8 => system.cpu.cop0.baddr = data,
            12 => system.cpu.cop0.sr = data,
            // Only bits 8 and 9 are writable
            13 => system.cpu.cop0.cause = (system.cpu.cop0.cause & !0x300) | (data & 0x300),
            _ => panic!("Unhandled cop0r{cop_r} write <- {data:x}"),
        }
        Ok(())
    }

    /// Move from cop0 register
    pub fn mfc0(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let cpu_r = instr.rt();
        let cop_r = instr.rd();

        let data = match cop_r {
            3 | 5 | 6 | 7 | 9 | 11 | 15 => 0,
            8 => system.cpu.cop0.baddr,
            12 => system.cpu.cop0.sr,
            13 => system.cpu.cop0.cause,
            14 => system.cpu.cop0.epc,
            _ => panic!("Unhandled cop0 register read {cop_r}"),
        };

        system.cpu.take_delayed_load(cpu_r, data);
        Ok(())
    }

    /// Return from exception
    pub fn rfe(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        if instr.sec() != 0x10 {
            panic!("Invalid cop0 instruction: {}", instr.0);
        }

        let mode = system.cpu.cop0.sr & 0x3F;
        system.cpu.cop0.sr = (system.cpu.cop0.sr & !0xF) | (mode >> 2);
        Ok(())
    }
}
