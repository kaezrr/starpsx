mod cop0;
mod instrs;
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
        let mut regs = [0xDEADBEEF; 32];
        regs[0] = 0;

        Cpu {
            regs,
            regd: regs,
            pc: 0xBFC00000,
            hi: 0,
            lo: 0,
            bus,
            next_instr: Opcode(0x0),
            load: (0, 0),
            cop0: Cop0::new(),
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
        println!("{:08X?} {:08X?}", instr.0, self.pc);
        self.decode_instruction(instr);

        self.regs = self.regd;
    }

    fn decode_instruction(&mut self, instr: Opcode) {
        match instr.pri() {
            0x00 => match instr.sec() {
                0x00 => self.sll(instr),
                0x25 => self.or(instr),
                0x2B => self.sltu(instr),
                0x21 => self.addu(instr),
                0x08 => self.jr(instr),
                0x24 => self.and(instr),
                0x20 => self.add(instr),
                0x09 => self.jalr(instr),
                0x03 => self.sra(instr),
                0x23 => self.subu(instr),
                _ => panic!("Unknown special instruction {:#08X}", instr.0),
            },
            0x0A => self.slti(instr),
            0x01 => self.bxxx(instr),
            0x24 => self.lbu(instr),
            0x06 => self.blez(instr),
            0x07 => self.bgtz(instr),
            0x04 => self.beq(instr),
            0x10 => self.cop0(instr),
            0x20 => self.lb(instr),
            0x03 => self.jal(instr),
            0x02 => self.j(instr),
            0x05 => self.bne(instr),
            0x08 => self.addi(instr),
            0x0C => self.andi(instr),
            0x28 => self.sb(instr),
            0x09 => self.addiu(instr),
            0x2B => self.sw(instr),
            0x0D => self.ori(instr),
            0x0F => self.lui(instr),
            0x23 => self.lw(instr),
            0x29 => self.sh(instr),
            _ => panic!("Unknown instruction {:#08X}", instr.0),
        }
    }
}
