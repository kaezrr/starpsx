use super::*;

impl Cpu {
    // Load and store instructions

    /// Load byte
    pub fn lb(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regs[rs].wrapping_add(im);
        let data = system.read::<u8>(addr)? as i8;

        system.cpu.take_delayed_load(rt, data as u32);
        Ok(())
    }

    /// Load byte unsigned
    pub fn lbu(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regs[rs].wrapping_add(im);
        let data = system.read::<u8>(addr)?;

        system.cpu.take_delayed_load(rt, data as u32);
        Ok(())
    }

    /// Load half word
    pub fn lh(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regs[rs].wrapping_add(im);
        let data = system.read::<u16>(addr)? as i16;

        system.cpu.take_delayed_load(rt, data as u32);
        Ok(())
    }

    /// Load half word unsigned
    pub fn lhu(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regs[rs].wrapping_add(im);
        let data = system.read::<u16>(addr)?;

        system.cpu.take_delayed_load(rt, data as u32);
        Ok(())
    }

    /// Load word
    pub fn lw(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        if system.cpu.cop0.sr & 0x10000 != 0 {
            eprintln!("ignoring load while cache is isolated");
            return Ok(());
        }

        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regs[rs].wrapping_add(im);
        let data = system.read::<u32>(addr)?;

        system.cpu.take_delayed_load(rt, data);
        Ok(())
    }

    /// Store byte
    pub fn sb(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        if system.cpu.cop0.sr & 0x10000 != 0 {
            eprintln!("ignoring store while cache is isolated");
            return Ok(());
        }
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regs[rs].wrapping_add(im);
        let data = system.cpu.regs[rt] as u8;

        system.write::<u8>(addr, data)
    }

    /// Store half word
    pub fn sh(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        if system.cpu.cop0.sr & 0x10000 != 0 {
            eprintln!("ignoring store while cache is isolated");
            return Ok(());
        }
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regs[rs].wrapping_add(im);
        let data = system.cpu.regs[rt] as u16;

        system.write::<u16>(addr, data)?;
        Ok(())
    }

    /// Store word
    pub fn sw(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        if system.cpu.cop0.sr & 0x10000 != 0 {
            eprintln!("ignoring store while cache is isolated");
            return Ok(());
        }

        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regs[rs].wrapping_add(im);
        let data = system.cpu.regs[rt];

        system.write::<u32>(addr, data)?;
        Ok(())
    }

    /// Unaligned left word load
    pub fn lwl(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regd[rs].wrapping_add(im);
        let val = system.cpu.regd[rt];

        let aligned_addr = addr & !3;
        let word = system.read::<u32>(aligned_addr)?;

        let data = match addr & 3 {
            0 => (val & 0x00FFFFFF) | (word << 24),
            1 => (val & 0x0000FFFF) | (word << 16),
            2 => (val & 0x000000FF) | (word << 8),
            3 => word,
            _ => unreachable!(),
        };

        system.cpu.take_delayed_load(rt, data as u32);
        Ok(())
    }

    /// Unaligned right word load
    pub fn lwr(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regd[rs].wrapping_add(im);
        let val = system.cpu.regd[rt];

        let aligned_addr = addr & !3;
        let word = system.read::<u32>(aligned_addr)?;

        let data = match addr & 3 {
            0 => word,
            1 => (val & 0xFF000000) | (word >> 8),
            2 => (val & 0xFFFF0000) | (word >> 16),
            3 => (val & 0xFFFFFF00) | (word >> 24),
            _ => unreachable!(),
        };

        system.cpu.take_delayed_load(rt, data as u32);
        Ok(())
    }

    /// Unaligned left word store
    pub fn swl(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regs[rs].wrapping_add(im);
        let val = system.cpu.regs[rt];

        let aligned_addr = addr & !3;
        let word = system.read::<u32>(aligned_addr)?;

        let data = match addr & 3 {
            0 => (word & 0xFFFFFF00) | (val >> 24),
            1 => (word & 0xFFFF0000) | (val >> 16),
            2 => (word & 0xFF000000) | (val >> 8),
            3 => val,
            _ => unreachable!(),
        };

        system.write::<u32>(aligned_addr, data)?;
        Ok(())
    }

    /// Unaligned right word store
    pub fn swr(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = system.cpu.regs[rs].wrapping_add(im);
        let val = system.cpu.regs[rt];

        let aligned_addr = addr & !3;
        let word = system.read::<u32>(aligned_addr)?;

        let data = match addr & 3 {
            0 => val,
            1 => (word & 0x000000FF) | (val << 8),
            2 => (word & 0x0000FFFF) | (val << 16),
            3 => (word & 0x00FFFFFF) | (val << 24),
            _ => unreachable!(),
        };

        system.write::<u32>(aligned_addr, data)?;
        Ok(())
    }

    // ALU instructions

    /// rd = rs + rt (overflow trap)
    pub fn add(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rs] as i32;
        let rhs = system.cpu.regs[rt] as i32;

        system.cpu.regd[rd] = match lhs.checked_add(rhs) {
            Some(v) => v as u32,
            None => return Err(Exception::Overflow),
        };
        Ok(())
    }

    /// rd = rs + rt
    pub fn addu(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rs];
        let rhs = system.cpu.regs[rt];

        system.cpu.regd[rd] = lhs.wrapping_add(rhs);
        Ok(())
    }

    /// rd = rs - rt (overflow trap)
    pub fn sub(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rs] as i32;
        let rhs = system.cpu.regs[rt] as i32;

        system.cpu.regd[rd] = match lhs.checked_sub(rhs) {
            Some(v) => v as u32,
            None => return Err(Exception::Overflow),
        };

        Ok(())
    }

    /// rd = rs - rt
    pub fn subu(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rs];
        let rhs = system.cpu.regs[rt];

        system.cpu.regd[rd] = lhs.wrapping_sub(rhs);
        Ok(())
    }

    /// rd = rs + imm (overflow trap)
    pub fn addi(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = system.cpu.regs[rs] as i32;
        let rhs = im as i32;

        system.cpu.regd[rt] = match lhs.checked_add(rhs) {
            Some(v) => v as u32,
            None => return Err(Exception::Overflow),
        };
        Ok(())
    }

    /// rd = rs + imm
    pub fn addiu(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = system.cpu.regs[rs];
        let rhs = im as i32;

        system.cpu.regd[rt] = lhs.wrapping_add_signed(rhs);
        Ok(())
    }

    /// rd = rs < rt
    pub fn slt(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rs] as i32;
        let rhs = system.cpu.regs[rt] as i32;

        system.cpu.regd[rd] = (lhs < rhs) as u32;
        Ok(())
    }

    /// rd = rs < rt (unsigned)
    pub fn sltu(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rs];
        let rhs = system.cpu.regs[rt];

        system.cpu.regd[rd] = (lhs < rhs) as u32;
        Ok(())
    }

    /// rd = rs < imm
    pub fn slti(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = system.cpu.regs[rs] as i32;
        let rhs = im as i32;

        system.cpu.regd[rt] = (lhs < rhs) as u32;
        Ok(())
    }

    /// rd = rs < imm (unsigned)
    pub fn sltiu(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = system.cpu.regs[rs];
        let rhs = im;

        system.cpu.regd[rt] = (lhs < rhs) as u32;
        Ok(())
    }

    /// rd = rs AND rt
    pub fn and(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rs];
        let rhs = system.cpu.regs[rt];

        system.cpu.regd[rd] = lhs & rhs;
        Ok(())
    }

    /// rd = rs OR rt
    pub fn or(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rs];
        let rhs = system.cpu.regs[rt];

        system.cpu.regd[rd] = lhs | rhs;
        Ok(())
    }

    /// rd = rs XOR rt
    pub fn xor(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rs];
        let rhs = system.cpu.regs[rt];

        system.cpu.regd[rd] = lhs ^ rhs;
        Ok(())
    }

    /// rd = 0xFFFFFFFF XOR (rs OR rt)
    pub fn nor(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rs];
        let rhs = system.cpu.regs[rt];

        system.cpu.regd[rd] = 0xFFFFFFFF ^ (lhs | rhs);
        Ok(())
    }

    /// rt = rs AND imm
    pub fn andi(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16();

        let lhs = system.cpu.regs[rs];
        let rhs = im;

        system.cpu.regd[rt] = lhs & rhs;
        Ok(())
    }

    /// rt = rs OR imm
    pub fn ori(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16();

        let lhs = system.cpu.regs[rs];
        let rhs = im;

        system.cpu.regd[rt] = lhs | rhs;
        Ok(())
    }

    /// rt = rs XOR imm
    pub fn xori(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16();

        let lhs = system.cpu.regs[rs];
        let rhs = im;

        system.cpu.regd[rt] = lhs ^ rhs;
        Ok(())
    }

    /// rd = rt SHL (rs AND 1Fh)
    pub fn sllv(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rt];
        let rhs = system.cpu.regs[rs];

        system.cpu.regd[rd] = lhs << (rhs & 0x1F);
        Ok(())
    }

    /// rd = rt SHR (rs AND 1Fh)
    pub fn srlv(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rt];
        let rhs = system.cpu.regs[rs];

        system.cpu.regd[rd] = lhs >> (rhs & 0x1F);
        Ok(())
    }

    /// rd = rt SAR (rs AND 1Fh)
    pub fn srav(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = system.cpu.regs[rt] as i32;
        let rhs = system.cpu.regs[rs];

        system.cpu.regd[rd] = lhs.wrapping_shr(rhs) as u32;
        Ok(())
    }

    /// rd = rt SHL imm
    pub fn sll(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = system.cpu.regs[rt];
        let rhs = im;

        system.cpu.regd[rd] = lhs.wrapping_shl(rhs);
        Ok(())
    }

    /// rd = rt SHR imm
    pub fn srl(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = system.cpu.regs[rt];
        let rhs = im;

        system.cpu.regd[rd] = lhs.unbounded_shr(rhs);
        Ok(())
    }

    /// rd = rt SAR imm
    pub fn sra(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = system.cpu.regs[rt] as i32;
        let rhs = im;

        system.cpu.regd[rd] = lhs.unbounded_shr(rhs) as u32;
        Ok(())
    }

    /// rt = imm << 16
    pub fn lui(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let im = instr.imm16();

        system.cpu.regd[rt] = im << 16;
        Ok(())
    }

    /// hi:lo = rs * rt (signed)
    pub fn mult(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = system.cpu.regs[rs] as i32 as i64;
        let rhs = system.cpu.regs[rt] as i32 as i64;

        let res = lhs * rhs;

        system.cpu.hi = (res >> 32) as u32;
        system.cpu.lo = res as u32;
        Ok(())
    }

    /// hi:lo = rs * rt (unsigned)
    pub fn multu(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = system.cpu.regs[rs] as u64;
        let rhs = system.cpu.regs[rt] as u64;

        let res = lhs * rhs;

        system.cpu.hi = (res >> 32) as u32;
        system.cpu.lo = res as u32;
        Ok(())
    }

    /// hi:lo = rs / rt (signed)
    pub fn div(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = system.cpu.regs[rs] as i32;
        let rhs = system.cpu.regs[rt] as i32;

        let (quo, rem) = match rhs {
            -1 if lhs == i32::MIN => (i32::MIN, 0),
            0 => (if lhs >= 0 { -1 } else { 1 }, lhs),
            _ => (lhs / rhs, lhs % rhs),
        };

        system.cpu.hi = rem as u32;
        system.cpu.lo = quo as u32;
        Ok(())
    }

    /// hi:lo = rs / rt (unsigned)
    pub fn divu(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = system.cpu.regs[rs];
        let rhs = system.cpu.regs[rt];

        let (quo, rem) = match rhs {
            0 => (u32::MAX, lhs),
            _ => (lhs / rhs, lhs % rhs),
        };

        system.cpu.hi = rem;
        system.cpu.lo = quo;
        Ok(())
    }

    /// Move from hi
    pub fn mfhi(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rd = instr.rd();
        system.cpu.regd[rd] = system.cpu.hi;
        Ok(())
    }

    /// Move from lo
    pub fn mflo(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rd = instr.rd();
        system.cpu.regd[rd] = system.cpu.lo;
        Ok(())
    }

    /// Move to hi
    pub fn mthi(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rs = instr.rs();
        system.cpu.hi = system.cpu.regs[rs];
        Ok(())
    }

    /// Move to lo
    pub fn mtlo(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rs = instr.rs();
        system.cpu.lo = system.cpu.regs[rs];
        Ok(())
    }

    // Branching instructions

    /// Jump
    pub fn j(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let im = instr.imm26();
        let addr = (system.cpu.pc & 0xF0000000) + (im << 2);

        system.cpu.delayed_branch = Some(addr);
        Ok(())
    }

    /// Jump and link
    pub fn jal(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let im = instr.imm26();
        let addr = (system.cpu.pc & 0xF0000000) + (im << 2);

        system.cpu.regd[31] = system.cpu.pc.wrapping_add(8);
        system.cpu.delayed_branch = Some(addr);
        Ok(())
    }

    /// Jump from register address
    pub fn jr(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rs = instr.rs();
        let addr = system.cpu.regs[rs];

        system.cpu.delayed_branch = Some(addr);
        Ok(())
    }

    /// Jump from register address and link
    pub fn jalr(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rs = instr.rs();
        let rd = instr.rd();
        let addr = system.cpu.regs[rs];

        system.cpu.regd[rd] = system.cpu.pc.wrapping_add(8);
        system.cpu.delayed_branch = Some(addr);
        Ok(())
    }

    /// Branch if equal
    pub fn beq(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rs = instr.rs();
        let rt = instr.rt();
        let im = instr.imm16_se();

        if system.cpu.regs[rs] == system.cpu.regs[rt] {
            let addr = system.cpu.pc.wrapping_add((im << 2).wrapping_add(4));
            system.cpu.delayed_branch = Some(addr);
        }
        Ok(())
    }

    /// Branch if not equal
    pub fn bne(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rs = instr.rs();
        let rt = instr.rt();
        let im = instr.imm16_se();

        if system.cpu.regs[rs] != system.cpu.regs[rt] {
            let addr = system.cpu.pc.wrapping_add((im << 2).wrapping_add(4));
            system.cpu.delayed_branch = Some(addr);
        }
        Ok(())
    }

    /// Handles BLTZ, BGEZ, BLTZAL, BGEZAL after decoding the opcode
    pub fn bxxx(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rs = instr.rs();
        let im = instr.imm16_se();

        let ge = (instr.0 >> 16) & 1 == 1;
        let al = (instr.0 >> 17) & 0xF == 0x8;

        let cond = ((system.cpu.regs[rs] as i32) < 0) ^ ge;
        if al {
            system.cpu.regd[31] = system.cpu.pc.wrapping_add(8);
        }
        if cond {
            let addr = system.cpu.pc.wrapping_add((im << 2).wrapping_add(4));
            system.cpu.delayed_branch = Some(addr);
        }
        Ok(())
    }

    /// Branch if greater than zero
    pub fn bgtz(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rs = instr.rs();
        let im = instr.imm16_se();

        if (system.cpu.regs[rs] as i32) > 0 {
            let addr = system.cpu.pc.wrapping_add((im << 2).wrapping_add(4));
            system.cpu.delayed_branch = Some(addr);
        }
        Ok(())
    }

    /// Branch if less than or equal to zero
    pub fn blez(system: &mut System, instr: Instruction) -> Result<(), Exception> {
        let rs = instr.rs();
        let im = instr.imm16_se();

        if (system.cpu.regs[rs] as i32) <= 0 {
            let addr = system.cpu.pc.wrapping_add((im << 2).wrapping_add(4));
            system.cpu.delayed_branch = Some(addr);
        }
        Ok(())
    }

    pub fn syscall() -> Result<(), Exception> {
        Err(Exception::Syscall)
    }

    pub fn breakk() -> Result<(), Exception> {
        Err(Exception::Break)
    }

    pub fn cop2(_system: &mut System, instr: Instruction) -> Result<(), Exception> {
        todo!("GTE instruction {:x}", instr.0);
    }

    pub fn lwc2(_system: &mut System, instr: Instruction) -> Result<(), Exception> {
        todo!("GTE load word {:x}", instr.0);
    }

    pub fn swc2(_system: &mut System, instr: Instruction) -> Result<(), Exception> {
        todo!("GTE store word {:x}", instr.0);
    }

    pub fn cop1() -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn cop3() -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn lwc0() -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn lwc1() -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn lwc3() -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn swc0() -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn swc1() -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn swc3() -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }
}
