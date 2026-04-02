mod cop0;
mod gte;
mod instrs;
pub mod utils;

use cop0::Cop0;
use tracing::error;
use utils::Exception;
use utils::Instruction;

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
        let mut regs = [0xDEAD_BEEF; 32];
        regs[0] = 0;
        let pc = 0xBFC0_0000;

        Self {
            regs,
            regd: regs,
            pc,
            hi: 0xDEAD_BEEF,
            lo: 0xDEAD_BEEF,
            load: None,
            delayed_branch: None,
            cop0: Cop0::default(),
            gte: GTEngine::default(),
        }
    }
}

impl Cpu {
    pub fn run_next_instruction(system: &mut System) {
        let instr = Instruction(match system.read::<4>(system.cpu.pc) {
            Ok(v) => v,
            Err(e) => return system.cpu.handle_exception(&e, false),
        });

        let (next_pc, in_delay) = match system.cpu.delayed_branch.take() {
            Some(addr) => (addr, true),
            None => (system.cpu.pc.wrapping_add(4), false),
        };

        let is_gte = (instr.0 & 0xFE00_0000) == 0x4A00_0000;
        let interrupt_pending = Self::pending_interrupts(system);

        // Skip execution when an interrupt is pending, UNLESS it's a GTE
        // instruction outside a branch delay slot
        if !interrupt_pending || (is_gte && !in_delay) {
            let cpu = &mut system.cpu;
            // Consume delay slot and set it, except if its zero register
            match cpu.load.take() {
                Some((reg, val)) if reg != 0 => cpu.regd[reg] = val,
                _ => (),
            }

            if let Err(exception) = Self::execute_opcode(system, instr) {
                system.cpu.handle_exception(&exception, in_delay);
                return;
            }

            let cpu = &mut system.cpu;
            cpu.regs = cpu.regd;
            cpu.regs[0] = 0;
            system.cpu.pc = next_pc;
        }

        if interrupt_pending {
            system.cpu.handle_exception(&Exception::Interrupt, in_delay);
        }
    }

    const fn pending_interrupts(system: &mut System) -> bool {
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

    const fn handle_exception(&mut self, cause: &Exception, branch: bool) {
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
        if let Exception::LoadAddressError(x) | Exception::StoreAddressError(x) = cause {
            self.cop0.baddr = *x;
        }

        // Exception handler address based on BEV field of Cop0 SR
        self.pc = if (self.cop0.sr >> 22) & 1 == 1 {
            0xBFC0_0180
        } else {
            0x8000_0080
        };
    }

    const fn take_delayed_load(&mut self, rt: usize, data: u32) {
        // If there was already a pending load to this register, then cancel it.
        if self.regd[rt] != self.regs[rt] {
            self.regd[rt] = self.regs[rt];
        }
        self.load = Some((rt, data));
    }

    fn execute_opcode(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        match instr.pri() {
            0x00 => match instr.sec() {
                0x00 => Self::sll(system, instr),
                0x02 => Self::srl(system, instr),
                0x03 => Self::sra(system, instr),
                0x04 => Self::sllv(system, instr),
                0x06 => Self::srlv(system, instr),
                0x07 => Self::srav(system, instr),
                0x08 => Self::jr(system, instr),
                0x09 => Self::jalr(system, instr),
                0x0C => Self::syscall()?,
                0x0D => Self::breakk()?,
                0x10 => Self::mfhi(system, instr),
                0x11 => Self::mthi(system, instr),
                0x12 => Self::mflo(system, instr),
                0x13 => Self::mtlo(system, instr),
                0x18 => Self::mult(system, instr),
                0x19 => Self::multu(system, instr),
                0x1A => Self::div(system, instr),
                0x1B => Self::divu(system, instr),
                0x20 => Self::add(system, instr)?,
                0x21 => Self::addu(system, instr),
                0x22 => Self::sub(system, instr)?,
                0x23 => Self::subu(system, instr),
                0x24 => Self::and(system, instr),
                0x25 => Self::or(system, instr),
                0x26 => Self::xor(system, instr),
                0x27 => Self::nor(system, instr),
                0x2A => Self::slt(system, instr),
                0x2B => Self::sltu(system, instr),
                _ => {
                    error!("Illegal instruction {:08x}", instr.0);
                    return Err(Exception::IllegalInstruction);
                }
            },
            0x01 => Self::bxxx(system, instr),
            0x02 => Self::j(system, instr),
            0x03 => Self::jal(system, instr),
            0x04 => Self::beq(system, instr),
            0x05 => Self::bne(system, instr),
            0x06 => Self::blez(system, instr),
            0x07 => Self::bgtz(system, instr),
            0x08 => Self::addi(system, instr)?,
            0x09 => Self::addiu(system, instr),
            0x0A => Self::slti(system, instr),
            0x0B => Self::sltiu(system, instr),
            0x0C => Self::andi(system, instr),
            0x0D => Self::ori(system, instr),
            0x0E => Self::xori(system, instr),
            0x0F => Self::lui(system, instr),
            0x11 => Self::cop1()?,
            0x13 => Self::cop3()?,
            0x20 => Self::lb(system, instr)?,
            0x21 => Self::lh(system, instr)?,
            0x22 => Self::lwl(system, instr)?,
            0x23 => Self::lw(system, instr)?,
            0x24 => Self::lbu(system, instr)?,
            0x25 => Self::lhu(system, instr)?,
            0x26 => Self::lwr(system, instr)?,
            0x28 => Self::sb(system, instr)?,
            0x29 => Self::sh(system, instr)?,
            0x2A => Self::swl(system, instr)?,
            0x2B => Self::sw(system, instr)?,
            0x2E => Self::swr(system, instr)?,
            0x30 => Self::lwc0()?,
            0x31 => Self::lwc1()?,
            0x33 => Self::lwc3()?,
            0x38 => Self::swc0()?,
            0x39 => Self::swc1()?,
            0x3B => Self::swc3()?,

            0x32 => gte::lwc2(system, instr)?,
            0x12 => gte::cop2(system, instr)?,
            0x3A => gte::swc2(system, instr)?,
            0x10 => cop0::cop0(system, instr),

            _ => {
                error!("Illegal instruction {:08x}", instr.0);
                return Err(Exception::IllegalInstruction);
            }
        }

        Ok(())
    }

    pub const fn snapshot(&self) -> Snapshot {
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
