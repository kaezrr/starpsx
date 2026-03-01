mod cop0;
mod gte;
mod instrs;
pub mod utils;

use cop0::Cop0;
use tracing::error;
use utils::{Exception, Instruction};

use crate::System;
use crate::cpu::gte::GTEngine;

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

    /// Geometry Transformation Engine (Coprocessor 2)
    gte: GTEngine,
}

impl Default for Cpu {
    fn default() -> Self {
        let mut regs = [0xDEADBEEF; 32];
        regs[0] = 0;
        let pc = 0xBFC00000;

        Self {
            regs,
            regd: regs,
            pc,
            hi: 0xDEADBEEF,
            lo: 0xDEADBEEF,
            load: None,
            delayed_branch: None,
            cop0: Cop0::default(),
            gte: GTEngine::default(),
        }
    }
}

impl Cpu {
    pub fn run_next_instruction(system: &mut System) {
        let instr = Instruction(match system.read::<u32>(system.cpu.pc) {
            Ok(v) => v,
            Err(e) => return system.cpu.handle_exception(e, false),
        });

        let (next_pc, in_delay) = match system.cpu.delayed_branch.take() {
            Some(addr) => (addr, true),
            None => (system.cpu.pc.wrapping_add(4), false),
        };

        let is_gte = (instr.0 & 0xFE000000) == 0x4A000000;
        let interrupt_pending = Cpu::pending_interrupts(system);

        // Skip execution when an interrupt is pending, UNLESS it's a GTE
        // instruction outside a branch delay slot
        if !interrupt_pending || (is_gte && !in_delay) {
            let cpu = &mut system.cpu;
            // Consume delay slot and set it, except if its zero register
            match cpu.load.take() {
                Some((reg, val)) if reg != 0 => cpu.regd[reg] = val,
                _ => (),
            }

            if let Err(exception) = Cpu::execute_opcode(system, instr) {
                system.cpu.handle_exception(exception, in_delay);
                return;
            }

            let cpu = &mut system.cpu;
            cpu.regs = cpu.regd;
            cpu.regs[0] = 0;
            system.cpu.pc = next_pc;
        }

        if interrupt_pending {
            system.cpu.handle_exception(Exception::Interrupt, in_delay);
        }
    }

    #[inline(always)]
    fn pending_interrupts(system: &mut System) -> bool {
        let cpu = &mut system.cpu;

        // Bit 10 of cause corresponds to any pending external interrupts
        if system.irqctl.pending() {
            cpu.cop0.cause |= 1 << 10;
        } else {
            cpu.cop0.cause &= !(1 << 10);
        }

        // Mask Bit 10 and Bit 9 - 8 (Software Interrrupts) with SR
        let pending = (cpu.cop0.cause & cpu.cop0.sr) & 0x700;
        pending != 0 && (cpu.cop0.sr & 1 != 0)
    }

    #[inline(always)]
    fn handle_exception(&mut self, cause: Exception, branch: bool) {
        // SR shifting
        let mode = self.cop0.sr & 0x3F;
        self.cop0.sr = (self.cop0.sr & !0x3F) | (mode << 2 & 0x3F);

        // Set the exception code
        self.cop0.cause &= !0x7c;
        self.cop0.cause |= cause.code() << 2;

        // Check if currently in Branch Delay Slot
        if branch {
            self.cop0.epc = self.pc.wrapping_sub(4);
            self.cop0.cause |= 1 << 31;
        } else {
            self.cop0.epc = self.pc;
            self.cop0.cause &= !(1 << 31);
        }

        // If bad address exception then store that in cop0r8
        match cause {
            Exception::LoadAddressError(x) => self.cop0.baddr = x,
            Exception::StoreAddressError(x) => self.cop0.baddr = x,
            _ => (),
        }

        // Exception handler address based on BEV field of Cop0 SR
        self.pc = match (self.cop0.sr >> 22) & 1 == 1 {
            true => 0xBFC00180,
            false => 0x80000080,
        };
    }

    #[inline(always)]
    fn take_delayed_load(&mut self, rt: usize, data: u32) {
        // If there was already a pending load to this register, then cancel it.
        if self.regd[rt] != self.regs[rt] {
            self.regd[rt] = self.regs[rt];
        }
        self.load = Some((rt, data));
    }

    fn execute_opcode(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        match instr.pri() {
            0x00 => match instr.sec() {
                0x00 => Cpu::sll(system, instr),
                0x02 => Cpu::srl(system, instr),
                0x03 => Cpu::sra(system, instr),
                0x04 => Cpu::sllv(system, instr),
                0x06 => Cpu::srlv(system, instr),
                0x07 => Cpu::srav(system, instr),
                0x08 => Cpu::jr(system, instr),
                0x09 => Cpu::jalr(system, instr),
                0x0C => Cpu::syscall(),
                0x0D => Cpu::breakk(),
                0x10 => Cpu::mfhi(system, instr),
                0x11 => Cpu::mthi(system, instr),
                0x12 => Cpu::mflo(system, instr),
                0x13 => Cpu::mtlo(system, instr),
                0x18 => Cpu::mult(system, instr),
                0x19 => Cpu::multu(system, instr),
                0x1A => Cpu::div(system, instr),
                0x1B => Cpu::divu(system, instr),
                0x20 => Cpu::add(system, instr),
                0x21 => Cpu::addu(system, instr),
                0x22 => Cpu::sub(system, instr),
                0x23 => Cpu::subu(system, instr),
                0x24 => Cpu::and(system, instr),
                0x25 => Cpu::or(system, instr),
                0x26 => Cpu::xor(system, instr),
                0x27 => Cpu::nor(system, instr),
                0x2A => Cpu::slt(system, instr),
                0x2B => Cpu::sltu(system, instr),
                _ => {
                    error!("Illegal instruction {:08x}", instr.0);
                    Err(Exception::IllegalInstruction)
                }
            },
            0x01 => Cpu::bxxx(system, instr),
            0x02 => Cpu::j(system, instr),
            0x03 => Cpu::jal(system, instr),
            0x04 => Cpu::beq(system, instr),
            0x05 => Cpu::bne(system, instr),
            0x06 => Cpu::blez(system, instr),
            0x07 => Cpu::bgtz(system, instr),
            0x08 => Cpu::addi(system, instr),
            0x09 => Cpu::addiu(system, instr),
            0x0A => Cpu::slti(system, instr),
            0x0B => Cpu::sltiu(system, instr),
            0x0C => Cpu::andi(system, instr),
            0x0D => Cpu::ori(system, instr),
            0x0E => Cpu::xori(system, instr),
            0x0F => Cpu::lui(system, instr),
            0x10 => cop0::cop0(system, instr),
            0x11 => Cpu::cop1(),
            0x12 => gte::cop2(system, instr),
            0x13 => Cpu::cop3(),
            0x20 => Cpu::lb(system, instr),
            0x21 => Cpu::lh(system, instr),
            0x22 => Cpu::lwl(system, instr),
            0x23 => Cpu::lw(system, instr),
            0x24 => Cpu::lbu(system, instr),
            0x25 => Cpu::lhu(system, instr),
            0x26 => Cpu::lwr(system, instr),
            0x28 => Cpu::sb(system, instr),
            0x29 => Cpu::sh(system, instr),
            0x2A => Cpu::swl(system, instr),
            0x2B => Cpu::sw(system, instr),
            0x2E => Cpu::swr(system, instr),
            0x30 => Cpu::lwc0(),
            0x31 => Cpu::lwc1(),
            0x32 => gte::lwc2(system, instr),
            0x33 => Cpu::lwc3(),
            0x38 => Cpu::swc0(),
            0x39 => Cpu::swc1(),
            0x3A => gte::swc2(system, instr),
            0x3B => Cpu::swc3(),
            _ => {
                error!("Illegal instruction {:08x}", instr.0);
                Err(Exception::IllegalInstruction)
            }
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            pc: self.pc,
            hi: self.hi,
            lo: self.lo,
            regs: self.regs,
        }
    }
}

pub struct Snapshot {
    pub pc: u32,
    pub hi: u32,
    pub lo: u32,

    pub regs: [u32; 32],
}
