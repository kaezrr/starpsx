pub struct DebugSnapshot {
    // CPU
    pub pc: u32,
    lo: u32,
    hi: u32,
    cpu_regs: [u32; 32],

    instr_slice: [(u32, u32); 200],
}

impl Default for DebugSnapshot {
    fn default() -> Self {
        Self {
            pc: 0x112c,
            lo: Default::default(),
            hi: Default::default(),
            cpu_regs: Default::default(),
            instr_slice: std::array::from_fn(|i| (i as u32 * 4 + 0x1000, 0)),
        }
    }
}

impl DebugSnapshot {
    pub fn get_cpu_state(&self) -> Vec<(&str, u32)> {
        let mut registers = Vec::with_capacity(35);

        for (i, v) in self.cpu_regs.into_iter().enumerate() {
            registers.push((super::disasm::reg_name(i as u8), v));
        }

        registers.push(("hi", self.hi));
        registers.push(("lo", self.lo));
        registers.push(("pc", self.pc));

        registers
    }

    pub fn get_disassembly(&self) -> Vec<(u32, u32, String)> {
        let mut disasm = Vec::with_capacity(200);

        for (addr, instr) in self.instr_slice {
            let d = super::disasm::decode_instruction(instr, addr);
            disasm.push((addr, instr, d));
        }

        disasm
    }
}
