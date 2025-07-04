mod cop0;
mod instrs;
pub mod utils;

use crate::memory::Bus;
use cop0::Cop0;
use utils::{Exception, Opcode};

pub struct Cpu {
    /// 32-bit general purpose registers, R0 is always zero
    pub regs: [u32; 32],

    /// Register copies for delay slot emulation
    regd: [u32; 32],

    /// Program counter
    pub pc: u32,

    /// Delayed branch slot
    delayed_branch: Option<u32>,

    /// Upper 32 bits of product or division remainder
    hi: u32,

    /// Lower 32 bits of product or division quotient
    lo: u32,

    /// Load to execute
    load: Option<(usize, u32)>,

    /// Coprocessor 0
    cop0: Cop0,
}

impl Cpu {
    pub fn new() -> Self {
        let mut regs = [0xDEADBEEF; 32];
        regs[0] = 0;
        let pc = 0xBFC00000;

        Cpu {
            regs,
            regd: regs,
            pc,
            hi: 0xDEADBEEF,
            lo: 0xDEADBEEF,
            load: None,
            delayed_branch: None,
            cop0: Cop0::new(),
        }
    }

    /// Run a single instruction and return the number of cycles
    pub fn run_instruction(&mut self, bus: &mut Bus) {
        let instr = Opcode(match bus.read32(self.pc) {
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
        if let Err(exception) = self.execute_opcode(instr, bus) {
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
        self.cop0.epc = if branch { self.pc - 4 } else { self.pc };

        self.pc = handler;
    }

    fn cop2(&mut self, instr: Opcode) -> Result<(), Exception> {
        panic!("Unhandled GTE instruction {:x}", instr.0);
    }

    fn execute_opcode(&mut self, instr: Opcode, bus: &mut Bus) -> Result<(), Exception> {
        match instr.pri() {
            0x00 => match instr.sec() {
                0x00 => self.sll(instr),
                0x02 => self.srl(instr),
                0x03 => self.sra(instr),
                0x04 => self.sllv(instr),
                0x06 => self.srlv(instr),
                0x07 => self.srav(instr),
                0x08 => self.jr(instr),
                0x09 => self.jalr(instr),
                0x0C => self.syscall(),
                0x0D => self.breakk(),
                0x10 => self.mfhi(instr),
                0x11 => self.mthi(instr),
                0x12 => self.mflo(instr),
                0x13 => self.mtlo(instr),
                0x18 => self.mult(instr),
                0x19 => self.multu(instr),
                0x1A => self.div(instr),
                0x1B => self.divu(instr),
                0x20 => self.add(instr),
                0x21 => self.addu(instr),
                0x22 => self.sub(instr),
                0x23 => self.subu(instr),
                0x24 => self.and(instr),
                0x25 => self.or(instr),
                0x26 => self.xor(instr),
                0x27 => self.nor(instr),
                0x2A => self.slt(instr),
                0x2B => self.sltu(instr),
                _ => Err(Exception::IllegalInstruction),
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
            0x0E => self.xori(instr),
            0x0F => self.lui(instr),
            0x10 => self.cop0(instr),
            0x11 => self.cop1(),
            0x12 => self.cop2(instr),
            0x13 => self.cop3(),
            0x20 => self.lb(instr, bus),
            0x21 => self.lh(instr, bus),
            0x22 => self.lwl(instr, bus),
            0x23 => self.lw(instr, bus),
            0x24 => self.lbu(instr, bus),
            0x25 => self.lhu(instr, bus),
            0x26 => self.lwr(instr, bus),
            0x28 => self.sb(instr, bus),
            0x29 => self.sh(instr, bus),
            0x2A => self.swl(instr, bus),
            0x2B => self.sw(instr, bus),
            0x2E => self.swr(instr, bus),
            0x30 => self.lwc0(),
            0x31 => self.lwc1(),
            0x32 => self.lwc2(instr),
            0x33 => self.lwc3(),
            0x38 => self.swc0(),
            0x39 => self.swc1(),
            0x3A => self.swc2(instr),
            0x3B => self.swc3(),
            _ => Err(Exception::IllegalInstruction),
        }
    }
}
