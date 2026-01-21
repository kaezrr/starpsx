/// Map a 5‑bit register number to its conventional name.
pub fn reg_name(r: u8) -> &'static str {
    match r {
        0 => "zero",
        1 => "at",
        2 => "v0",
        3 => "v1",
        4 => "a0",
        5 => "a1",
        6 => "a2",
        7 => "a3",
        8 => "t0",
        9 => "t1",
        10 => "t2",
        11 => "t3",
        12 => "t4",
        13 => "t5",
        14 => "t6",
        15 => "t7",
        16 => "s0",
        17 => "s1",
        18 => "s2",
        19 => "s3",
        20 => "s4",
        21 => "s5",
        22 => "s6",
        23 => "s7",
        24 => "t8",
        25 => "t9",
        26 => "k0",
        27 => "k1",
        28 => "gp",
        29 => "sp",
        30 => "fp",
        31 => "ra",
        _ => unreachable!("invalid cpu reg"),
    }
}

/// Decode a 32‑bit instruction into a human‑readable string.
pub fn decode_instruction(instr: u32, pc: u32) -> String {
    let op = ((instr >> 26) & 0x3F) as u8;
    let rs = ((instr >> 21) & 0x1F) as u8;
    let rt = ((instr >> 16) & 0x1F) as u8;
    let rd = ((instr >> 11) & 0x1F) as u8;
    let shamt = ((instr >> 6) & 0x1F) as u8;
    let funct = (instr & 0x3F) as u8;
    let imm = (instr & 0xFFFF) as u16;
    let simm = imm as i16 as u32; // sign‑extend
    let target = instr & 0x03FF_FFFF; // 26‑bit jump
    let target = (pc & 0xF0000000) + (target << 2);

    match op {
        0x00 => {
            // SPECIAL: use `funct`
            match funct {
                0x00 => format!("sll {}, {}, {}", reg_name(rd), reg_name(rt), shamt),
                0x02 => format!("srl {}, {}, {}", reg_name(rd), reg_name(rt), shamt),
                0x03 => format!("sra {}, {}, {}", reg_name(rd), reg_name(rt), shamt),
                0x04 => format!("sllv {}, {}, {}", reg_name(rd), reg_name(rt), reg_name(rs)),
                0x06 => format!("srlv {}, {}, {}", reg_name(rd), reg_name(rt), reg_name(rs)),
                0x07 => format!("srav {}, {}, {}", reg_name(rd), reg_name(rt), reg_name(rs)),
                0x08 => format!("jr {}", reg_name(rs)),
                0x09 => format!("jalr {}, {}", reg_name(rd), reg_name(rs)),
                0x0C => "syscall".to_string(),
                0x0D => "break".to_string(),
                0x10 => format!("mfhi {}", reg_name(rd)),
                0x11 => format!("mthi {}", reg_name(rs)),
                0x12 => format!("mflo {}", reg_name(rd)),
                0x13 => format!("mtlo {}", reg_name(rs)),
                0x18 => format!("mult {}, {}", reg_name(rs), reg_name(rt)),
                0x19 => format!("multu {}, {}", reg_name(rs), reg_name(rt)),
                0x1A => format!("div {}, {}", reg_name(rs), reg_name(rt)),
                0x1B => format!("divu {}, {}", reg_name(rs), reg_name(rt)),
                0x20 => format!("add {}, {}, {}", reg_name(rd), reg_name(rs), reg_name(rt)),
                0x21 => format!("addu {}, {}, {}", reg_name(rd), reg_name(rs), reg_name(rt)),
                0x22 => format!("sub {}, {}, {}", reg_name(rd), reg_name(rs), reg_name(rt)),
                0x23 => format!("subu {}, {}, {}", reg_name(rd), reg_name(rs), reg_name(rt)),
                0x24 => format!("and {}, {}, {}", reg_name(rd), reg_name(rs), reg_name(rt)),
                0x25 => format!("or {}, {}, {}", reg_name(rd), reg_name(rs), reg_name(rt)),
                0x26 => format!("xor {}, {}, {}", reg_name(rd), reg_name(rs), reg_name(rt)),
                0x27 => format!("nor {}, {}, {}", reg_name(rd), reg_name(rs), reg_name(rt)),
                0x2A => format!("slt {}, {}, {}", reg_name(rd), reg_name(rs), reg_name(rt)),
                0x2B => format!("sltu {}, {}, {}", reg_name(rd), reg_name(rs), reg_name(rt)),
                other => format!("UNKNOWN_FUNCT {other:#02X}"),
            }
        }

        0x01 => {
            // REGIMM: rt distinguishes
            match rt {
                0x00 => format!("bltz {}, {}", reg_name(rs), simm),
                0x01 => format!("bgez {}, {}", reg_name(rs), simm),
                0x10 => format!("bltzal {}, {}", reg_name(rs), simm),
                0x11 => format!("bgezal {}, {}", reg_name(rs), simm),
                other => format!("UNKNOWN_REGIMM rt={other:#02X}"),
            }
        }

        0x02 => format!("j {target:#08x}"),
        0x03 => format!("jal {target:#08x}"),

        0x04 => format!(
            "beq {}, {}, {:#x}",
            reg_name(rs),
            reg_name(rt),
            pc.wrapping_add(4 + (simm << 2))
        ),
        0x05 => format!(
            "bne {}, {}, {:#x}",
            reg_name(rs),
            reg_name(rt),
            pc.wrapping_add(4 + (simm << 2))
        ),
        0x06 => format!(
            "blez {}, {:#x}",
            reg_name(rs),
            pc.wrapping_add(4 + (simm << 2))
        ),
        0x07 => format!(
            "bgtz {}, {:#x}",
            reg_name(rs),
            pc.wrapping_add(4 + (simm << 2))
        ),

        0x08 => format!("addi {}, {}, {}", reg_name(rt), reg_name(rs), simm as i16),
        0x09 => format!("addiu {}, {}, {}", reg_name(rt), reg_name(rs), simm as i16),
        0x0A => format!("slti {}, {}, {}", reg_name(rt), reg_name(rs), simm),
        0x0B => format!("sltiu {}, {}, {}", reg_name(rt), reg_name(rs), simm),
        0x0C => format!("andi {}, {}, {:#06x}", reg_name(rt), reg_name(rs), imm),
        0x0D => format!("ori {}, {}, {:#06x}", reg_name(rt), reg_name(rs), imm),
        0x0E => format!("xori {}, {}, {:#06x}", reg_name(rt), reg_name(rs), imm),
        0x0F => format!("lui {}, {:#06x}", reg_name(rt), imm),

        // COP0 – use `rs` to dispatch
        0x10 => match rs {
            0x00 => format!("mfc0 {}, {}", reg_name(rt), reg_name(rd)),
            0x04 => format!("mtc0 {}, {}", reg_name(rt), reg_name(rd)),
            0x10 => "rfe".to_string(),
            other => format!("UNKNOWN_COP0 rs={other:#02X}"),
        },

        0x11 => "cop1 (FP)".to_string(),
        0x12 => "cop2".to_string(),
        0x13 => "cop3".to_string(),

        // Loads
        0x20 => format!("lb {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),
        0x21 => format!("lh {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),
        0x22 => format!("lwl {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),
        0x23 => format!("lw {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),
        0x24 => format!("lbu {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),
        0x25 => format!("lhu {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),
        0x26 => format!("lwr {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),

        // Stores
        0x28 => format!("sb {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),
        0x29 => format!("sh {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),
        0x2A => format!("swl {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),
        0x2B => format!("sw {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),
        0x2E => format!("swr {}, {:#x}({})", reg_name(rt), simm as i16, reg_name(rs)),

        _ => format!("UNKNOWN_OP {op:#02X}"),
    }
}
