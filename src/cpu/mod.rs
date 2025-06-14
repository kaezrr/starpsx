mod opcode;

use crate::memory::Bus;
use opcode::Opcode;

struct Cpu {
    /// 32-bit general purpose registers, R0 is always zero
    regs: [u32; 32],

    /// Program counter
    pc: u32,

    /// Upper 32 bits of product or division remainder
    hi: u32,

    /// Lower 32 bits of product or division quotient
    lo: u32,

    /// Bus interface
    bus: Bus,
}

impl Cpu {
    fn new(bus: Bus) -> Self {
        let regs = [0xDEADBEEF; 32];

        // Bios entry point
        let pc = 0xBFC00000;
        let hi = 0;
        let lo = 0;

        Cpu {
            regs,
            pc,
            hi,
            lo,
            bus,
        }
    }

    /// Run a single instruction and return the number of cycles
    fn run_instruction(&mut self) {
        let pc = self.pc;

        // Fetch opcode and increment program counter
        let instr = Opcode::new(self.bus.read32(pc));
        self.pc = pc.wrapping_add(4);

        // Decode and run the instruction
        self.decode_instruction(instr);

        // Register 0 is always 0
        self.regs[0] = 0x0;
    }

    fn decode_instruction(&mut self, op: Opcode) {
        match op.pri() {
            0x00 => match op.sec() {
                0x00 => (),
                0x01 => (),
                0x02 => (),
                0x03 => (),
                0x04 => (),
                0x05 => (),
                0x06 => (),
                0x07 => (),
                0x08 => (),
                0x09 => (),
                0x0A => (),
                0x0B => (),
                0x0C => (),
                0x0D => (),
                0x0E => (),
                0x0F => (),
                0x10 => (),
                0x11 => (),
                0x12 => (),
                0x13 => (),
                0x14 => (),
                0x15 => (),
                0x16 => (),
                0x17 => (),
                0x18 => (),
                0x19 => (),
                0x1A => (),
                0x1B => (),
                0x1C => (),
                0x1D => (),
                0x1E => (),
                0x1F => (),
                0x20 => (),
                0x21 => (),
                0x22 => (),
                0x23 => (),
                0x24 => (),
                0x25 => (),
                0x26 => (),
                0x27 => (),
                0x28 => (),
                0x29 => (),
                0x2A => (),
                0x2B => (),
                0x2C => (),
                0x2D => (),
                0x2E => (),
                0x2F => (),
                0x30 => (),
                0x31 => (),
                0x32 => (),
                0x33 => (),
                0x34 => (),
                0x35 => (),
                0x36 => (),
                0x37 => (),
                0x38 => (),
                0x39 => (),
                0x3A => (),
                0x3B => (),
                0x3C => (),
                0x3D => (),
                0x3E => (),
                0x3F => (),
                _ => panic!("Unknown special instruction {:x}", op.sec()),
            },
            0x01 => (),
            0x02 => (),
            0x03 => (),
            0x04 => (),
            0x05 => (),
            0x06 => (),
            0x07 => (),
            0x08 => (),
            0x09 => (),
            0x0A => (),
            0x0B => (),
            0x0C => (),
            0x0D => (),
            0x0E => (),
            0x0F => (),
            0x10 => (),
            0x11 => (),
            0x12 => (),
            0x13 => (),
            0x14 => (),
            0x15 => (),
            0x16 => (),
            0x17 => (),
            0x18 => (),
            0x19 => (),
            0x1A => (),
            0x1B => (),
            0x1C => (),
            0x1D => (),
            0x1E => (),
            0x1F => (),
            0x20 => (),
            0x21 => (),
            0x22 => (),
            0x23 => (),
            0x24 => (),
            0x25 => (),
            0x26 => (),
            0x27 => (),
            0x28 => (),
            0x29 => (),
            0x2A => (),
            0x2B => (),
            0x2C => (),
            0x2D => (),
            0x2E => (),
            0x2F => (),
            0x30 => (),
            0x31 => (),
            0x32 => (),
            0x33 => (),
            0x34 => (),
            0x35 => (),
            0x36 => (),
            0x37 => (),
            0x38 => (),
            0x39 => (),
            0x3A => (),
            0x3B => (),
            0x3C => (),
            0x3D => (),
            0x3E => (),
            0x3F => (),
            _ => panic!("Unknown instruction {:x}", op.pri()),
        }
    }

    // Load and store instructions

    /// Load byte
    fn lb(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read8(addr) as i8;

        self.regs[rt] = data as u32;
    }

    /// Load byte unsigned
    fn lbu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read8(addr);

        self.regs[rt] = data as u32;
    }

    /// Load half word
    fn lh(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read16(addr) as i16;

        self.regs[rt] = data as u32;
    }

    /// Load half word unsigned
    fn lhu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read16(addr);

        self.regs[rt] = data as u32;
    }

    /// Load word
    fn lw(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read32(addr);

        self.regs[rt] = data;
    }

    /// Store byte
    fn sb(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.regs[rt] as u8;

        self.bus.write8(addr, data);
    }

    /// Store half word
    fn sh(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.regs[rt] as u16;

        self.bus.write16(addr, data);
    }

    /// Store word
    fn sw(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.regs[rt];

        self.bus.write32(addr, data);
    }

    /// Unaligned left word load
    fn lwl(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let val = self.regs[rt];

        let aligned_addr = addr & !3;
        let word = self.bus.read32(aligned_addr);

        let data = match addr & 3 {
            0 => (val & 0x00FFFFFF) | (word << 24),
            1 => (val & 0x0000FFFF) | (word << 16),
            2 => (val & 0x000000FF) | (word << 8),
            3 => word,
            _ => unreachable!(),
        };

        self.regs[rt] = data;
    }

    /// Unaligned right word load
    fn lwr(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let val = self.regs[rt];

        let aligned_addr = addr & !3;
        let word = self.bus.read32(aligned_addr);

        let data = match addr & 3 {
            0 => word,
            1 => (val & 0xFF000000) | (word >> 8),
            2 => (val & 0xFFFF0000) | (word >> 16),
            3 => (val & 0xFFFFFF00) | (word >> 24),
            _ => unreachable!(),
        };

        self.regs[rt] = data;
    }

    /// Unaligned left word store
    fn swl(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let val = self.regs[rt];

        let aligned_addr = addr & !3;
        let word = self.bus.read32(aligned_addr);

        let data = match addr & 3 {
            0 => (word & 0x00FFFFFF) | (val << 24),
            1 => (word & 0x0000FFFF) | (val << 16),
            2 => (word & 0x000000FF) | (val << 8),
            3 => val,
            _ => unreachable!(),
        };

        self.bus.write32(aligned_addr, data);
    }

    /// Unaligned right word store
    fn swr(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let val = self.regs[rt];

        let aligned_addr = addr & !3;
        let word = self.bus.read32(aligned_addr);

        let data = match addr & 3 {
            0 => val,
            1 => (word & 0xFF000000) | (val >> 8),
            2 => (word & 0xFFFF0000) | (val >> 16),
            3 => (word & 0xFFFFFF00) | (val >> 24),
            _ => unreachable!(),
        };

        self.bus.write32(aligned_addr, data);
    }

    // ALU instructions

    /// rd = rs + rt (overflow trap)
    pub fn add(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regs[rd] = lhs + rhs;
    }

    /// rd = rs + rt
    pub fn addu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regs[rd] = lhs + rhs;
    }

    /// rd = rs - rt (overflow trap)
    pub fn sub(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regs[rd] = lhs - rhs;
    }

    /// rd = rs - rt
    pub fn subu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regs[rd] = lhs - rhs;
    }

    /// rd = rs + imm (overflow trap)
    pub fn addi(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regs[rt] = lhs + rhs;
    }

    /// rd = rs + imm
    pub fn addiu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regs[rt] = lhs + rhs;
    }

    /// rd = rs < rt
    pub fn slt(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs] as i32;
        let rhs = self.regs[rt] as i32;

        self.regs[rd] = (lhs < rhs) as u32;
    }

    /// rd = rs < rt (unsigned)
    pub fn sltu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regs[rd] = (lhs < rhs) as u32;
    }

    /// rd = rs < imm
    pub fn slti(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs] as i32;
        let rhs = im as i32;

        self.regs[rt] = (lhs < rhs) as u32;
    }

    /// rd = rs < imm (unsigned)
    pub fn sltiu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regs[rt] = (lhs < rhs) as u32;
    }

    /// rd = rs AND rt
    pub fn and(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regs[rd] = lhs & rhs;
    }

    /// rd = rs OR rt
    pub fn or(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regs[rd] = lhs | rhs;
    }

    /// rd = rs XOR rt
    pub fn xor(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regs[rd] = lhs ^ rhs;
    }

    /// rd = 0xFFFFFFFF XOR (rs OR rt)
    pub fn nor(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regs[rd] = 0xFFFFFFFF ^ (lhs | rhs);
    }

    /// rt = rs AND imm
    pub fn andi(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regs[rt] = lhs & rhs;
    }

    /// rt = rs OR imm
    pub fn ori(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regs[rt] = lhs | rhs;
    }

    /// rt = rs XOR imm
    pub fn xori(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regs[rt] = lhs ^ rhs;
    }

    /// rd = rt SHL (rs AND 1Fh)
    pub fn sllv(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rt];
        let rhs = self.regs[rs];

        self.regs[rd] = lhs.wrapping_shl(rhs);
    }

    /// rd = rt SHR (rs AND 1Fh)
    pub fn srlv(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rt];
        let rhs = self.regs[rs];

        self.regs[rd] = lhs.wrapping_shr(rhs);
    }

    /// rd = rt SAR (rs AND 1Fh)
    pub fn srav(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rt] as i32;
        let rhs = self.regs[rs];

        self.regs[rd] = lhs.wrapping_shr(rhs) as u32;
    }

    /// rd = rt SHL imm
    pub fn sll(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = self.regs[rt];
        let rhs = im;

        self.regs[rd] = lhs.wrapping_shl(rhs);
    }

    /// rd = rt SHR imm
    pub fn srl(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = self.regs[rt];
        let rhs = im;

        self.regs[rd] = lhs.wrapping_shr(rhs);
    }

    /// rd = rt SAR imm
    pub fn sra(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = self.regs[rt] as i32;
        let rhs = im;

        self.regs[rd] = lhs.wrapping_shr(rhs) as u32;
    }

    /// rt = imm << 16
    pub fn lui(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let im = instr.imm16();

        self.regs[rt] = im << 16;
    }

    /// hi:lo = rs * rt (signed)
    pub fn mult(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = self.regs[rs] as i64;
        let rhs = self.regs[rt] as i64;

        let res = (lhs * rhs) as u64;

        self.hi = (res >> 32) as u32;
        self.lo = res as u32;
    }

    /// hi:lo = rs * rt (unsigned)
    pub fn multu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = self.regs[rs] as u64;
        let rhs = self.regs[rt] as u64;

        let res = lhs * rhs;

        self.hi = (res >> 32) as u32;
        self.lo = res as u32;
    }

    /// hi:lo = rs / rt (signed)
    pub fn div(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = self.regs[rs] as i32;
        let rhs = self.regs[rt] as i32;

        let quo = (lhs / rhs) as u32;
        let rem = (lhs % rhs) as u32;

        self.hi = rem;
        self.lo = quo;
    }

    /// hi:lo = rs / rt (unsigned)
    pub fn divu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        let quo = lhs / rhs;
        let rem = lhs % rhs;

        self.hi = rem;
        self.lo = quo;
    }

    /// Move from hi
    pub fn mfhi(&mut self, instr: Opcode) {
        let rd = instr.rd();
        self.regs[rd] = self.hi;
    }

    /// Move from lo
    pub fn mflo(&mut self, instr: Opcode) {
        let rd = instr.rd();
        self.regs[rd] = self.lo;
    }

    /// Move to hi
    pub fn mthi(&mut self, instr: Opcode) {
        let rs = instr.rs();
        self.hi = self.regs[rs];
    }

    /// Move to lo
    pub fn mtlo(&mut self, instr: Opcode) {
        let rs = instr.rs();
        self.lo = self.regs[rs];
    }

    // Branching instructions
}
