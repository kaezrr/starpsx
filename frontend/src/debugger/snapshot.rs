use starpsx_core::SystemSnapshot;

use crate::debugger::disasm::DisasmLine;
use crate::debugger::disasm::{self};

pub fn get_cpu_state(snapshot: &SystemSnapshot) -> [(&str, u32); 35] {
    std::array::from_fn(|i| match i {
        32 => ("hi", snapshot.cpu.hi),
        33 => ("lo", snapshot.cpu.lo),
        34 => ("pc", snapshot.cpu.pc),
        _ => (super::disasm::REG_NAME[i], snapshot.cpu.regs[i]),
    })
}

pub fn get_disassembly(snapshot: &SystemSnapshot) -> [(u32, u32, DisasmLine); 200] {
    std::array::from_fn(|i| {
        let (addr, instr) = snapshot.ins[i];
        let disasm = disasm::decode_instruction_line(instr, addr);
        (addr, instr, disasm)
    })
}
