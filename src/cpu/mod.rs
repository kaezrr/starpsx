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

    /// Current operation code
    op: Opcode,
}

impl Cpu {
    fn new(bus: Bus) -> Self {
        let regs = [0xDEADBEEF; 32];

        // Bios entry point
        let pc = 0xBFC00000;
        let hi = 0;
        let lo = 0;
        let op = Opcode::new();

        Cpu {
            regs,
            pc,
            hi,
            lo,
            bus,
            op,
        }
    }

    /// Run a single instruction and return the number of cycles
    fn run_instruction(&mut self) {
        let instr = self.bus.read32(self.pc);
        self.op.set(instr);
        self.pc = self.pc.wrapping_add(4);
        self.decode_instruction();

        // Register 0 is always 0
        self.regs[0] = 0x0;
    }

    fn decode_instruction(&mut self) {
        match self.op.pri() {
            0x00 => match self.op.sec() {
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
                _ => panic!("Unknown special instruction {:x}", self.op.sec()),
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
            _ => panic!("Unknown instruction {:x}", self.op.pri()),
        }
    }

    /// Load byte
    fn lb(&mut self) {
        let rt = self.op.rt();
        let rs = self.op.rs();
        let im = self.op.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read8(addr) as i8;

        self.regs[rt] = data as u32;
    }

    /// Load byte unsigned
    fn lbu(&mut self) {
        let rt = self.op.rt();
        let rs = self.op.rs();
        let im = self.op.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read8(addr);

        self.regs[rt] = data as u32;
    }

    /// Load half word
    fn lh(&mut self) {
        let rt = self.op.rt();
        let rs = self.op.rs();
        let im = self.op.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read16(addr) as i16;

        self.regs[rt] = data as u32;
    }

    /// Load half word unsigned
    fn lhu(&mut self) {
        let rt = self.op.rt();
        let rs = self.op.rs();
        let im = self.op.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read16(addr);

        self.regs[rt] = data as u32;
    }

    /// Load word
    fn lw(&mut self) {
        let rt = self.op.rt();
        let rs = self.op.rs();
        let im = self.op.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.bus.read32(addr);

        self.regs[rt] = data;
    }

    /// Store byte
    fn sb(&mut self) {
        let rt = self.op.rt();
        let rs = self.op.rs();
        let im = self.op.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.regs[rt] as u8;

        self.bus.write8(addr, data);
    }

    /// Store half word
    fn sh(&mut self) {
        let rt = self.op.rt();
        let rs = self.op.rs();
        let im = self.op.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.regs[rt] as u16;

        self.bus.write16(addr, data);
    }

    /// Store word
    fn sw(&mut self) {
        let rt = self.op.rt();
        let rs = self.op.rs();
        let im = self.op.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let data = self.regs[rt];

        self.bus.write32(addr, data);
    }

    /// Unaligned left word load
    fn lwl(&mut self) {
        let rt = self.op.rt();
        let rs = self.op.rs();
        let im = self.op.imm16_se();

        let addr = self.regs[rs].wrapping_add(im);
        let num_bytes = 4 - ((addr ^ 3) % 4);

        let mut data = self.regs[rt];
        for i in 0..num_bytes {
            let byte = self.bus.read8(addr + i) as u32;
            let mask = 0xFF << (i * 8);
            data = (data & mask) | (byte << ((3 - i) * 8));
        }

        self.regs[rt] = data;
    }

    /// Unaligned right word load
    fn lwr(&mut self) {}

    /// Unaligned left word store
    fn swl(&mut self) {}

    /// Unaligned right word store
    fn swr(&mut self) {}
}
