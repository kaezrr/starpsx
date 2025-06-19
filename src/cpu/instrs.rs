use super::*;

impl Cpu {
    // Load and store instructions

    /// Load byte
    pub fn lb(&mut self, instr: Opcode, bus: &Bus) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = bus.read8(addr) as i8;

        self.load = Some((rt, data as u32));
        Ok(())
    }

    /// Load byte unsigned
    pub fn lbu(&mut self, instr: Opcode, bus: &Bus) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = bus.read8(addr);

        self.load = Some((rt, data as u32));
        Ok(())
    }

    /// Load half word
    pub fn lh(&mut self, instr: Opcode, bus: &Bus) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = bus.read16(addr)? as i16;

        self.load = Some((rt, data as u32));
        Ok(())
    }

    /// Load half word unsigned
    pub fn lhu(&mut self, instr: Opcode, bus: &Bus) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = bus.read16(addr)?;

        self.load = Some((rt, data as u32));
        Ok(())
    }

    /// Load word
    pub fn lw(&mut self, instr: Opcode, bus: &Bus) -> Result<(), Exception> {
        if self.cop0.sr & 0x10000 != 0 {
            eprintln!("ignoring load while cache is isolated");
            return Ok(());
        }

        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = bus.read32(addr)?;

        self.load = Some((rt, data));
        Ok(())
    }

    /// Store byte
    pub fn sb(&mut self, instr: Opcode, bus: &mut Bus) -> Result<(), Exception> {
        if self.cop0.sr & 0x10000 != 0 {
            eprintln!("ignoring store while cache is isolated");
            return Ok(());
        }
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.regs[rt] as u8;

        bus.write8(addr, data);
        Ok(())
    }

    /// Store half word
    pub fn sh(&mut self, instr: Opcode, bus: &mut Bus) -> Result<(), Exception> {
        if self.cop0.sr & 0x10000 != 0 {
            eprintln!("ignoring store while cache is isolated");
            return Ok(());
        }
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.regs[rt] as u16;

        bus.write16(addr, data)?;
        Ok(())
    }

    /// Store word
    pub fn sw(&mut self, instr: Opcode, bus: &mut Bus) -> Result<(), Exception> {
        if self.cop0.sr & 0x10000 != 0 {
            eprintln!("ignoring store while cache is isolated");
            return Ok(());
        }

        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.regs[rt];

        bus.write32(addr, data)?;
        Ok(())
    }

    /// Unaligned left word load
    pub fn lwl(&mut self, instr: Opcode, bus: &Bus) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let val = self.regs[rt];

        let aligned_addr = addr & !3;
        let word = bus.read32(aligned_addr)?;

        let data = match addr & 3 {
            0 => (val & 0x00FFFFFF) | (word << 24),
            1 => (val & 0x0000FFFF) | (word << 16),
            2 => (val & 0x000000FF) | (word << 8),
            3 => word,
            _ => unreachable!(),
        };

        self.load = Some((rt, data));
        Ok(())
    }

    /// Unaligned right word load
    pub fn lwr(&mut self, instr: Opcode, bus: &Bus) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let val = self.regs[rt];

        let aligned_addr = addr & !3;
        let word = bus.read32(aligned_addr)?;

        let data = match addr & 3 {
            0 => word,
            1 => (val & 0xFF000000) | (word >> 8),
            2 => (val & 0xFFFF0000) | (word >> 16),
            3 => (val & 0xFFFFFF00) | (word >> 24),
            _ => unreachable!(),
        };

        self.load = Some((rt, data));
        Ok(())
    }

    /// Unaligned left word store
    pub fn swl(&mut self, instr: Opcode, bus: &mut Bus) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let val = self.regs[rt];

        let aligned_addr = addr & !3;
        let word = bus.read32(aligned_addr)?;

        let data = match addr & 3 {
            0 => (word & 0x00FFFFFF) | (val << 24),
            1 => (word & 0x0000FFFF) | (val << 16),
            2 => (word & 0x000000FF) | (val << 8),
            3 => val,
            _ => unreachable!(),
        };

        bus.write32(aligned_addr, data)?;
        Ok(())
    }

    /// Unaligned right word store
    pub fn swr(&mut self, instr: Opcode, bus: &mut Bus) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let val = self.regs[rt];

        let aligned_addr = addr & !3;
        let word = bus.read32(aligned_addr)?;

        let data = match addr & 3 {
            0 => val,
            1 => (word & 0xFF000000) | (val >> 8),
            2 => (word & 0xFFFF0000) | (val >> 16),
            3 => (word & 0xFFFFFF00) | (val >> 24),
            _ => unreachable!(),
        };

        bus.write32(aligned_addr, data)?;
        Ok(())
    }

    // ALU instructions

    /// rd = rs + rt (overflow trap)
    pub fn add(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = match lhs.checked_add(rhs) {
            Some(v) => v,
            None => return Err(Exception::Overflow),
        };
        Ok(())
    }

    /// rd = rs + rt
    pub fn addu(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs.wrapping_add(rhs);
        Ok(())
    }

    /// rd = rs - rt (overflow trap)
    pub fn sub(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = match lhs.checked_sub(rhs) {
            Some(v) => v,
            None => return Err(Exception::Overflow),
        };

        Ok(())
    }

    /// rd = rs - rt
    pub fn subu(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs.wrapping_sub(rhs);
        Ok(())
    }

    /// rd = rs + imm (overflow trap)
    pub fn addi(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im as i32;

        self.regd[rt] = match lhs.checked_add_signed(rhs) {
            Some(v) => v,
            None => return Err(Exception::Overflow),
        };
        Ok(())
    }

    /// rd = rs + imm
    pub fn addiu(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im as i32;

        self.regd[rt] = lhs.wrapping_add_signed(rhs);
        Ok(())
    }

    /// rd = rs < rt
    pub fn slt(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs] as i32;
        let rhs = self.regs[rt] as i32;

        self.regd[rd] = (lhs < rhs) as u32;
        Ok(())
    }

    /// rd = rs < rt (unsigned)
    pub fn sltu(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = (lhs < rhs) as u32;
        Ok(())
    }

    /// rd = rs < imm
    pub fn slti(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs] as i32;
        let rhs = im as i32;

        self.regd[rt] = (lhs < rhs) as u32;
        Ok(())
    }

    /// rd = rs < imm (unsigned)
    pub fn sltiu(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regd[rt] = (lhs < rhs) as u32;
        Ok(())
    }

    /// rd = rs AND rt
    pub fn and(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs & rhs;
        Ok(())
    }

    /// rd = rs OR rt
    pub fn or(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs | rhs;
        Ok(())
    }

    /// rd = rs XOR rt
    pub fn xor(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs ^ rhs;
        Ok(())
    }

    /// rd = 0xFFFFFFFF XOR (rs OR rt)
    pub fn nor(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = 0xFFFFFFFF ^ (lhs | rhs);
        Ok(())
    }

    /// rt = rs AND imm
    pub fn andi(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regd[rt] = lhs & rhs;
        Ok(())
    }

    /// rt = rs OR imm
    pub fn ori(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regd[rt] = lhs | rhs;
        Ok(())
    }

    /// rt = rs XOR imm
    pub fn xori(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regd[rt] = lhs ^ rhs;
        Ok(())
    }

    /// rd = rt SHL (rs AND 1Fh)
    pub fn sllv(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rt];
        let rhs = self.regs[rs];

        self.regd[rd] = lhs.unbounded_shl(rhs);
        Ok(())
    }

    /// rd = rt SHR (rs AND 1Fh)
    pub fn srlv(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rt];
        let rhs = self.regs[rs];

        self.regd[rd] = lhs.unbounded_shr(rhs);
        Ok(())
    }

    /// rd = rt SAR (rs AND 1Fh)
    pub fn srav(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rt] as i32;
        let rhs = self.regs[rs];

        self.regd[rd] = lhs.wrapping_shr(rhs) as u32;
        Ok(())
    }

    /// rd = rt SHL imm
    pub fn sll(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = self.regs[rt];
        let rhs = im;

        self.regd[rd] = lhs.wrapping_shl(rhs);
        Ok(())
    }

    /// rd = rt SHR imm
    pub fn srl(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = self.regs[rt];
        let rhs = im;

        self.regd[rd] = lhs.unbounded_shr(rhs);
        Ok(())
    }

    /// rd = rt SAR imm
    pub fn sra(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = self.regs[rt] as i32;
        let rhs = im;

        self.regd[rd] = lhs.unbounded_shr(rhs) as u32;
        Ok(())
    }

    /// rt = imm << 16
    pub fn lui(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let im = instr.imm16();

        self.regd[rt] = im << 16;
        Ok(())
    }

    /// hi:lo = rs * rt (signed)
    pub fn mult(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = self.regs[rs] as i64;
        let rhs = self.regs[rt] as i64;

        let res = (lhs * rhs) as u64;

        self.hi = (res >> 32) as u32;
        self.lo = res as u32;
        Ok(())
    }

    /// hi:lo = rs * rt (unsigned)
    pub fn multu(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = self.regs[rs] as u64;
        let rhs = self.regs[rt] as u64;

        let res = lhs * rhs;

        self.hi = (res >> 32) as u32;
        self.lo = res as u32;
        Ok(())
    }

    /// hi:lo = rs / rt (signed)
    pub fn div(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = self.regs[rs] as i32;
        let rhs = self.regs[rt] as i32;

        let (quo, rem) = match rhs {
            -1 if lhs == i32::MIN => (i32::MIN, 0),
            0 => (if lhs > 0 { -1 } else { 1 }, lhs),
            _ => (lhs / rhs, lhs % rhs),
        };

        self.hi = rem as u32;
        self.lo = quo as u32;
        Ok(())
    }

    /// hi:lo = rs / rt (unsigned)
    pub fn divu(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        let (quo, rem) = match rhs {
            0 => (u32::MAX, lhs),
            _ => (lhs / rhs, lhs % rhs),
        };

        self.hi = rem;
        self.lo = quo;
        Ok(())
    }

    /// Move from hi
    pub fn mfhi(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rd = instr.rd();
        self.regd[rd] = self.hi;
        Ok(())
    }

    /// Move from lo
    pub fn mflo(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rd = instr.rd();
        self.regd[rd] = self.lo;
        Ok(())
    }

    /// Move to hi
    pub fn mthi(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rs = instr.rs();
        self.hi = self.regs[rs];
        Ok(())
    }

    /// Move to lo
    pub fn mtlo(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rs = instr.rs();
        self.lo = self.regs[rs];
        Ok(())
    }

    // Branching instructions

    /// Jump
    pub fn j(&mut self, instr: Opcode) -> Result<(), Exception> {
        let im = instr.imm26();
        let addr = (self.pc & 0xF0000000) + (im << 2);

        self.delayed_branch = Some(addr);
        Ok(())
    }

    /// Jump and link
    pub fn jal(&mut self, instr: Opcode) -> Result<(), Exception> {
        let im = instr.imm26();
        let addr = (self.pc & 0xF0000000) + (im << 2);

        self.regd[31] = self.pc.wrapping_add(8);
        self.delayed_branch = Some(addr);
        Ok(())
    }

    /// Jump from register address
    pub fn jr(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rs = instr.rs();
        let addr = self.regs[rs];

        self.delayed_branch = Some(addr);
        Ok(())
    }

    /// Jump from register address and link
    pub fn jalr(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rs = instr.rs();
        let rd = instr.rd();
        let addr = self.regs[rs];

        self.regd[rd] = self.pc.wrapping_add(8);
        self.delayed_branch = Some(addr);
        Ok(())
    }

    /// Branch if equal
    pub fn beq(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rs = instr.rs();
        let rt = instr.rt();
        let im = instr.imm16_se();
        let addr = self.pc.wrapping_add(4 + (im << 2));

        if self.regs[rs] == self.regs[rt] {
            self.delayed_branch = Some(addr);
        }
        Ok(())
    }

    /// Branch if not equal
    pub fn bne(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rs = instr.rs();
        let rt = instr.rt();
        let im = instr.imm16_se();
        let addr = self.pc.wrapping_add(4 + (im << 2));

        if self.regs[rs] != self.regs[rt] {
            self.delayed_branch = Some(addr);
        }
        Ok(())
    }

    /// Handles BLTZ, BGEZ, BLTZAL, BGEZAL after decoding the opcode
    pub fn bxxx(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rs = instr.rs();
        let im = instr.imm16_se();
        let addr = self.pc.wrapping_add(4 + (im << 2));

        let ge = (instr.0 >> 16) & 1 == 1;
        let al = (instr.0 >> 20) & 1 == 1;

        let cond = ((self.regs[rs] as i32) < 0) ^ ge;

        if cond {
            if al {
                self.regd[31] = self.pc.wrapping_add(8);
            }
            self.delayed_branch = Some(addr);
        }
        Ok(())
    }

    /// Branch if greater than zero
    pub fn bgtz(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rs = instr.rs();
        let im = instr.imm16_se();
        let addr = self.pc.wrapping_add(4 + (im << 2));

        if (self.regs[rs] as i32) > 0 {
            self.delayed_branch = Some(addr);
        }
        Ok(())
    }

    /// Branch if less than or equal to zero
    pub fn blez(&mut self, instr: Opcode) -> Result<(), Exception> {
        let rs = instr.rs();
        let im = instr.imm16_se();
        let addr = self.pc.wrapping_add(4 + (im << 2));

        if (self.regs[rs] as i32) <= 0 {
            self.delayed_branch = Some(addr);
        }
        Ok(())
    }

    pub fn syscall(&mut self) -> Result<(), Exception> {
        Err(Exception::Syscall)
    }

    pub fn breakk(&mut self) -> Result<(), Exception> {
        Err(Exception::Break)
    }

    /// Move to cop0 register
    pub fn mtc0(&mut self, instr: Opcode) -> Result<(), Exception> {
        let cpu_r = instr.rt();
        let cop_r = instr.rd();

        let data = self.regs[cpu_r];

        match cop_r {
            3 | 5 | 6 | 7 | 9 | 11 | 13 => {
                if data != 0 {
                    panic!("Unhandled write to cop0r{}", cop_r);
                }
            }
            12 => self.cop0.sr = data,
            _ => panic!("Unhandled cop0 register {cop_r}"),
        }
        Ok(())
    }

    /// Move from cop0 register
    pub fn mfc0(&mut self, instr: Opcode) -> Result<(), Exception> {
        let cpu_r = instr.rt();
        let cop_r = instr.rd();

        let data = match cop_r {
            12 => self.cop0.sr,
            13 => self.cop0.cause,
            14 => self.cop0.epc,
            _ => panic!("Unhandled cop0 register {cop_r}"),
        };

        self.load = Some((cpu_r, data));
        Ok(())
    }

    /// Return from exception
    pub fn rfe(&mut self, instr: Opcode) -> Result<(), Exception> {
        if instr.sec() != 0x10 {
            panic!("Invalid cop0 instruction: {}", instr.0);
        }

        let mode = self.cop0.sr & 0x3F;
        self.cop0.sr = self.cop0.sr & !0x3F | mode >> 2;
        Ok(())
    }

    pub fn lwc2(&mut self, instr: Opcode) -> Result<(), Exception> {
        panic!("Unhandled GTE load word {:x}", instr.0);
    }

    pub fn swc2(&mut self, instr: Opcode) -> Result<(), Exception> {
        panic!("Unhandled GTE store word {:x}", instr.0);
    }

    pub fn cop1(&mut self) -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn cop3(&mut self) -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn lwc0(&mut self) -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn lwc1(&mut self) -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn lwc3(&mut self) -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn swc0(&mut self) -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn swc1(&mut self) -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }

    pub fn swc3(&mut self) -> Result<(), Exception> {
        Err(Exception::CoprocessorError)
    }
}
