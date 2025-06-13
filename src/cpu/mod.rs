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

    fn lb(&mut self) {
        let rt = self.op.rt() as usize;
        let rs = self.op.rs() as usize;
        let im = self.op.imm16();

        let b = self.bus.read8(im + self.regs[rs]) as i8;

        self.regs[rt] = b as u32;
    }

    fn lbu(&mut self) {
        let rt = self.op.rt() as usize;
        let rs = self.op.rs() as usize;
        let im = self.op.imm16();

        let b = self.bus.read8(im + self.regs[rs]);

        self.regs[rt] = b as u32;
    }

    fn lh(&mut self) {
        let rt = self.op.rt() as usize;
        let rs = self.op.rs() as usize;
        let im = self.op.imm16();

        let h = self.bus.read16(im + self.regs[rs]) as i16;

        self.regs[rt] = h as u32;
    }

    fn lhu(&mut self) {
        let rt = self.op.rt() as usize;
        let rs = self.op.rs() as usize;
        let im = self.op.imm16();

        let h = self.bus.read16(im + self.regs[rs]);

        self.regs[rt] = h as u32;
    }

    fn lw(&mut self) {
        let rt = self.op.rt() as usize;
        let rs = self.op.rs() as usize;
        let im = self.op.imm16();

        let w = self.bus.read32(im + self.regs[rs]);

        self.regs[rt] = w;
    }

    fn sb(&mut self) {
        let rt = self.op.rt() as usize;
        let rs = self.op.rs() as usize;
        let im = self.op.imm16();
        self.bus.write8(self.regs[rs] + im, self.regs[rt] as u8);
    }

    fn sh(&mut self) {
        let rt = self.op.rt() as usize;
        let rs = self.op.rs() as usize;
        let im = self.op.imm16();
        self.bus.write16(self.regs[rs] + im, self.regs[rt] as u16);
    }

    fn sw(&mut self) {
        let rt = self.op.rt() as usize;
        let rs = self.op.rs() as usize;
        let im = self.op.imm16();
        self.bus.write32(self.regs[rs] + im, self.regs[rt]);
    }

    fn lwl(&mut self) {}

    fn lwr(&mut self) {}

    fn swl(&mut self) {}

    fn swr(&mut self) {}
}
