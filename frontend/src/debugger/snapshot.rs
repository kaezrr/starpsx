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
            _ => (super::disasm::reg_name(i as u8), self.cpu_regs[i]),
        })
    }

    pub fn get_disassembly(&self) -> [(u32, u32, String); 200] {
        std::array::from_fn(|i| {
            let (addr, instr) = self.instructions[i];
            let d = super::disasm::decode_instruction(instr, addr);
            (addr, instr, d)
        })
    }
}
