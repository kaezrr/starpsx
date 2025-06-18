mod cop0;
mod instrs;
pub mod utils;

use crate::memory::Bus;
use cop0::Cop0;
use utils::{Exception, Opcode};

pub struct Cpu {
    /// 32-bit general purpose registers, R0 is always zero
    regs: [u32; 32],

    /// Register copies for delay slot emulation
    regd: [u32; 32],

    /// Program counter
    pc: u32,

    /// Delayed branch slot
    delayed_branch: Option<u32>,

    /// Upper 32 bits of product or division remainder
    hi: u32,

    /// Lower 32 bits of product or division quotient
    lo: u32,

    /// Bus interface
    bus: Bus,

    /// Load to execute
    load: Option<(usize, u32)>,

    /// Coprocessor 0
    cop0: Cop0,
}

impl Cpu {
    pub fn new(bus: Bus) -> Self {
        let mut regs = [0xDEADBEEF; 32];
        regs[0] = 0;
        let pc = 0xBFC00000;

        Cpu {
            regs,
            regd: regs,
            pc,
            hi: 0xDEADBEEF,
            lo: 0xDEADBEEF,
            bus,
            load: None,
            delayed_branch: None,
            cop0: Cop0::new(),
        }
    }

    /// Run a single instruction and return the number of cycles
    pub fn run_instruction(&mut self) {
        let instr = Opcode(match self.bus.read32(self.pc) {
            Ok(v) => v,
            Err(e) => return self.handle_exception(e, false),
        });

        let (next_pc, in_delay_slot) = match self.delayed_branch.take() {
            Some(addr) => (addr, true),
            None => (self.pc.wrapping_add(4), false),
        };

        // Execute any pending loads
        match self.load.take() {
            Some((0, _)) => (),
            Some((reg, val)) => self.regd[reg] = val,
            None => (),
        }

        // Decode and run the instruction
        // println!("{:08X} {:08X}", self.pc, instr.0);
        if let Err(exception) = self.execute_opcode(instr) {
            self.handle_exception(exception, in_delay_slot);
            return;
        };
        self.regs = self.regd;

        // Increment program counter
        self.pc = next_pc;
    }

    fn handle_exception(&mut self, cause: Exception, branch: bool) {
        // Exception handler address based on BEV field of Cop0 SR
        let handler: u32 = match (self.cop0.sr >> 22) & 1 == 1 {
            true => 0xBFC00180,
            false => 0x80000080,
        };

        let mode = self.cop0.sr & 0x3F;
        self.cop0.sr = (self.cop0.sr & !0x3F) | (mode << 2 & 0x3F);

        self.cop0.cause = (cause as u32) << 2 | (branch as u32) << 31;
        self.cop0.epc = self.pc;

        self.pc = handler;
    }

    fn execute_opcode(&mut self, instr: Opcode) -> Result<(), Exception> {
        match instr.pri() {
            0x00 => match instr.sec() {
                0x00 => self.sll(instr),
                0x02 => self.srl(instr),
                0x03 => self.sra(instr),
                0x08 => self.jr(instr),
                0x09 => self.jalr(instr),
                0x0C => self.syscall(instr),
                0x10 => self.mfhi(instr),
                0x11 => self.mthi(instr),
                0x12 => self.mflo(instr),
                0x13 => self.mtlo(instr),
                0x1A => self.div(instr),
                0x1B => self.divu(instr),
                0x20 => self.add(instr),
                0x21 => self.addu(instr),
                0x23 => self.subu(instr),
                0x24 => self.and(instr),
                0x25 => self.or(instr),
                0x2A => self.slt(instr),
                0x2B => self.sltu(instr),
                _ => panic!("Unknown special instruction {:#08X}", instr.0),
            },
            0x01 => self.bxxx(instr),
            0x02 => self.j(instr),
            0x03 => self.jal(instr),
            0x04 => self.beq(instr),
            0x05 => self.bne(instr),
            0x06 => self.blez(instr),
            0x07 => self.bgtz(instr),
            0x08 => self.addi(instr),
            0x09 => self.addiu(instr),
            0x0A => self.slti(instr),
            0x0B => self.sltiu(instr),
            0x0C => self.andi(instr),
            0x0D => self.ori(instr),
            0x0F => self.lui(instr),
            0x10 => self.cop0(instr),
            0x20 => self.lb(instr),
            0x23 => self.lw(instr),
            0x24 => self.lbu(instr),
            0x25 => self.lhu(instr),
            0x28 => self.sb(instr),
            0x29 => self.sh(instr),
            0x2B => self.sw(instr),
            _ => panic!("Unknown instruction {:#08X}", instr.0),
        }
    }
}
