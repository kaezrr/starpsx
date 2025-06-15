mod cop0;
mod opcode;

use crate::memory::Bus;
use cop0::Cop0;
use opcode::Opcode;

pub struct Cpu {
    /// 32-bit general purpose registers, R0 is always zero
    regs: [u32; 32],

    /// Register copies for delay slot emulation
    regd: [u32; 32],

    /// Program counter
    pc: u32,

    /// Upper 32 bits of product or division remainder
    hi: u32,

    /// Lower 32 bits of product or division quotient
    lo: u32,

    /// Bus interface
    bus: Bus,

    /// Fetched instruction in pipeline
    next_instr: Opcode,

    /// Load to execute
    load: (usize, u32),

    /// Coprocessor 0
    cop0: Cop0,
}

impl Cpu {
    pub fn new(bus: Bus) -> Self {
        Cpu {
            regs: [0xDEADBEEF; 32],
            regd: [0xDEADBEEF; 32],
            pc: 0xBFC00000,
            hi: 0,
            lo: 0,
            bus,
            next_instr: Opcode(0x0),
            load: (0, 0),
            cop0: Cop0 {},
        }
    }

    /// Run a single instruction and return the number of cycles
    pub fn run_instruction(&mut self) {
        let pc = self.pc;

        let instr = self.next_instr;

        // Fetch opcode and increment program counter
        self.next_instr = Opcode(self.bus.read32(pc));
        self.pc = pc.wrapping_add(4);

        // Execute any pending loads
        let (reg, val) = self.load;
        self.regd[reg] = val;
        self.regd[0x0] = 0x0;
        self.load = (0x0, 0x0);

        // Decode and run the instruction
        self.decode_instruction(instr);

        self.regs = self.regd;
    }

    fn decode_instruction(&mut self, instr: Opcode) {
        match instr.pri() {
            0x00 => match instr.sec() {
                0x00 => self.sll(instr),
                _ => panic!("Unknown special instruction 0x{:08X}", instr.0),
            },
            0x0D => self.ori(instr),
            0x0F => self.lui(instr),
            _ => panic!("Unknown instruction 0x{:08X}", instr.0),
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

        self.load = (rt, data as u32);
    }

    /// Load byte unsigned
    fn lbu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read8(addr);

        self.load = (rt, data as u32);
    }

    /// Load half word
    fn lh(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read16(addr) as i16;

        self.load = (rt, data as u32);
    }

    /// Load half word unsigned
    fn lhu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read16(addr);

        self.load = (rt, data as u32);
    }

    /// Load word
    fn lw(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read32(addr);

        self.load = (rt, data);
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

        self.regd[rt] = data;
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

        self.regd[rt] = data;
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

        self.regd[rd] = lhs + rhs;
    }

    /// rd = rs + rt
    pub fn addu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs + rhs;
    }

    /// rd = rs - rt (overflow trap)
    pub fn sub(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs - rhs;
    }

    /// rd = rs - rt
    pub fn subu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs - rhs;
    }

    /// rd = rs + imm (overflow trap)
    pub fn addi(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regd[rt] = lhs + rhs;
    }

    /// rd = rs + imm
    pub fn addiu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regd[rt] = lhs + rhs;
    }

    /// rd = rs < rt
    pub fn slt(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs] as i32;
        let rhs = self.regs[rt] as i32;

        self.regd[rd] = (lhs < rhs) as u32;
    }

    /// rd = rs < rt (unsigned)
    pub fn sltu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = (lhs < rhs) as u32;
    }

    /// rd = rs < imm
    pub fn slti(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs] as i32;
        let rhs = im as i32;

        self.regd[rt] = (lhs < rhs) as u32;
    }

    /// rd = rs < imm (unsigned)
    pub fn sltiu(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regd[rt] = (lhs < rhs) as u32;
    }

    /// rd = rs AND rt
    pub fn and(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs & rhs;
    }

    /// rd = rs OR rt
    pub fn or(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs | rhs;
    }

    /// rd = rs XOR rt
    pub fn xor(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = lhs ^ rhs;
    }

    /// rd = 0xFFFFFFFF XOR (rs OR rt)
    pub fn nor(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rs];
        let rhs = self.regs[rt];

        self.regd[rd] = 0xFFFFFFFF ^ (lhs | rhs);
    }

    /// rt = rs AND imm
    pub fn andi(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regd[rt] = lhs & rhs;
    }

    /// rt = rs OR imm
    pub fn ori(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regd[rt] = lhs | rhs;
    }

    /// rt = rs XOR imm
    pub fn xori(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let im = instr.imm16_se();

        let lhs = self.regs[rs];
        let rhs = im;

        self.regd[rt] = lhs ^ rhs;
    }

    /// rd = rt SHL (rs AND 1Fh)
    pub fn sllv(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rt];
        let rhs = self.regs[rs];

        self.regd[rd] = lhs.wrapping_shl(rhs);
    }

    /// rd = rt SHR (rs AND 1Fh)
    pub fn srlv(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rt];
        let rhs = self.regs[rs];

        self.regd[rd] = lhs.wrapping_shr(rhs);
    }

    /// rd = rt SAR (rs AND 1Fh)
    pub fn srav(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rs = instr.rs();
        let rd = instr.rd();

        let lhs = self.regs[rt] as i32;
        let rhs = self.regs[rs];

        self.regd[rd] = lhs.wrapping_shr(rhs) as u32;
    }

    /// rd = rt SHL imm
    pub fn sll(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = self.regs[rt];
        let rhs = im;

        self.regd[rd] = lhs.wrapping_shl(rhs);
    }

    /// rd = rt SHR imm
    pub fn srl(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = self.regs[rt];
        let rhs = im;

        self.regd[rd] = lhs.wrapping_shr(rhs);
    }

    /// rd = rt SAR imm
    pub fn sra(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let rd = instr.rd();
        let im = instr.imm5();

        let lhs = self.regs[rt] as i32;
        let rhs = im;

        self.regd[rd] = lhs.wrapping_shr(rhs) as u32;
    }

    /// rt = imm << 16
    pub fn lui(&mut self, instr: Opcode) {
        let rt = instr.rt();
        let im = instr.imm16();

        self.regd[rt] = im << 16;
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
        self.regd[rd] = self.hi;
    }

    /// Move from lo
    pub fn mflo(&mut self, instr: Opcode) {
        let rd = instr.rd();
        self.regd[rd] = self.lo;
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

    pub fn j(&mut self, instr: Opcode) {
        let im = instr.imm26();
        let addr = (self.pc & 0xF0000000) + (im << 2);

        self.pc = addr;
    }

    pub fn jal(&mut self, instr: Opcode) {
        let im = instr.imm26();
        let addr = (self.pc & 0xF0000000) + (im << 2);

        self.regd[31] = self.pc;
        self.pc = addr;
    }

    pub fn jr(&mut self, instr: Opcode) {
        let rs = instr.rs();
        let addr = self.regs[rs];

        self.pc = addr;
    }

    pub fn jalr(&mut self, instr: Opcode) {
        let rs = instr.rs();
        let rd = instr.rd();
        let addr = self.regs[rs];

        self.regd[rd] = self.pc;
        self.pc = addr;
    }

    pub fn beq(&mut self, instr: Opcode) {
        let rs = instr.rs();
        let rt = instr.rt();
        let im = instr.imm16_se();
        let addr = self.pc + (im << 2);

        if self.regs[rs] == self.regs[rt] {
            self.pc = addr;
        }
    }

    pub fn bne(&mut self, instr: Opcode) {
        let rs = instr.rs();
        let rt = instr.rt();
        let im = instr.imm16_se();
        let addr = self.pc + (im << 2);

        if self.regs[rs] != self.regs[rt] {
            self.pc = addr;
        }
    }

    pub fn bltz(&mut self, instr: Opcode) {
        let rs = instr.rs();
        let im = instr.imm16_se();
        let addr = self.pc + (im << 2);

        if (self.regs[rs] as i32) < 0 {
            self.pc = addr;
        }
    }

    pub fn bgez(&mut self, instr: Opcode) {
        let rs = instr.rs();
        let im = instr.imm16_se();
        let addr = self.pc + (im << 2);

        if (self.regs[rs] as i32) >= 0 {
            self.pc = addr;
        }
    }

    pub fn bgtz(&mut self, instr: Opcode) {
        let rs = instr.rs();
        let im = instr.imm16_se();
        let addr = self.pc + (im << 2);

        if (self.regs[rs] as i32) > 0 {
            self.pc = addr;
        }
    }

    pub fn blez(&mut self, instr: Opcode) {
        let rs = instr.rs();
        let im = instr.imm16_se();
        let addr = self.pc + (im << 2);

        if (self.regs[rs] as i32) <= 0 {
            self.pc = addr;
        }
    }

    pub fn bltzal(&mut self, instr: Opcode) {
        let rs = instr.rs();
        let im = instr.imm16_se();
        let addr = self.pc + (im << 2);

        if (self.regs[rs] as i32) < 0 {
            self.regd[31] = self.pc;
            self.pc = addr;
        }
    }

    pub fn bgezal(&mut self, instr: Opcode) {
        let rs = instr.rs();
        let im = instr.imm16_se();
        let addr = self.pc + (im << 2);

        if (self.regs[rs] as i32) >= 0 {
            self.regd[31] = self.pc;
            self.pc = addr;
        }
    }

    pub fn syscall(&mut self, instr: Opcode) {
        print!("SYSCALL");
    }

    pub fn breakk(&mut self, instr: Opcode) {
        print!("BREAK");
    }
}
