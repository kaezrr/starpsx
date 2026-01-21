use crate::debugger::disasm::{self, DisasmLine};

pub struct DebugSnapshot {
    // CPU
    pub pc: u32,
    pub lo: u32,
    pub hi: u32,
    pub cpu_regs: [u32; 32],

    pub instructions: [(u32, u32); 200],
}

impl DebugSnapshot {
    pub fn get_cpu_state(&self) -> [(&str, u32); 35] {
        std::array::from_fn(|i| match i {
            32 => ("hi", self.hi),
            33 => ("lo", self.lo),
            34 => ("pc", self.pc),
            _ => (super::disasm::REG_NAME[i], self.cpu_regs[i]),
        })
    }

    pub fn get_disassembly(&self) -> [(u32, u32, DisasmLine); 200] {
        std::array::from_fn(|i| {
            let (addr, instr) = self.instructions[i];
            let disasm = disasm::decode_instruction_line(instr, addr);
            (addr, instr, disasm)
        })
    }
}
