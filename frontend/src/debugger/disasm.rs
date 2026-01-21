use eframe::egui::{self, Color32, RichText};

/// Map a 5‑bit register number to its conventional name.
pub const REG_NAME: [&str; 32] = [
    "zero", "at", "v0", "v1", "a0", "a1", "a2", "a3", "t0", "t1", "t2", "t3", "t4", "t5", "t6",
    "t7", "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "t8", "t9", "k0", "k1", "gp", "sp", "fp",
    "ra",
];

enum DisasmToken {
    Mnemonic(&'static str),
    RegisterName(&'static str),
    Literal(String),
}

#[derive(Default)]
pub struct DisasmLine {
    tokens: Vec<DisasmToken>,
}

impl DisasmLine {
    fn push_tokens(&mut self, tokens: Vec<DisasmToken>) {
        self.tokens.extend(tokens)
    }

    /// Draw disassembly as monospace highlighted text
    pub fn label_monospace(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            let is_dark = ui.visuals().dark_mode;

            let (mnemonic_color, register_color, literal_color) = if is_dark {
                (
                    Color32::from_rgb(100, 200, 255), // Bright cyan
                    Color32::from_rgb(150, 220, 150), // Bright green
                    Color32::from_rgb(255, 200, 100), // Bright orange
                )
            } else {
                (
                    Color32::from_rgb(0, 100, 180), // Dark blue
                    Color32::from_rgb(0, 130, 0),   // Dark green
                    Color32::from_rgb(180, 100, 0), // Dark orange/brown
                )
            };

            for (i, token) in self.tokens.iter().enumerate() {
                if i == 1 {
                    ui.label(RichText::new(" ").monospace());
                } else if i > 1 {
                    ui.label(RichText::new(", ").monospace());
                }

                match token {
                    DisasmToken::Mnemonic(text) => {
                        ui.label(RichText::new(*text).color(mnemonic_color).monospace());
                    }
                    DisasmToken::RegisterName(text) => {
                        ui.label(RichText::new(*text).color(register_color).monospace());
                    }
                    DisasmToken::Literal(text) => {
                        ui.label(RichText::new(text).color(literal_color).monospace());
                    }
                }
            }
        });
    }
}

/// Decode a 32-bit instruction into a `DisasmLine` (tokens).
pub fn decode_instruction_line(instr: u32, addr: u32) -> DisasmLine {
    let mut line = DisasmLine::default();

    let pri = ((instr >> 26) & 0x3F) as u8;
    let sec = (instr & 0x3F) as u8;
    let rs = ((instr >> 21) & 0x1F) as u8;
    let rt = ((instr >> 16) & 0x1F) as u8;
    let rd = ((instr >> 11) & 0x1F) as u8;
    let shamt = ((instr >> 6) & 0x1F) as u8;
    let imm = (instr & 0xFFFF) as u16;
    let simm = imm as i16;
    let target26 = instr & 0x03FF_FFFF;
    let target_addr = (addr & 0xF000_0000) | (target26 << 2);
    let branch_target = addr
        .wrapping_add(4)
        .wrapping_add(((simm as i32) << 2) as u32);

    let rsn = REG_NAME[rs as usize];
    let rtn = REG_NAME[rt as usize];
    let rdn = REG_NAME[rd as usize];

    match pri {
        0x00 => {
            // SPECIAL - use funct (sec)
            match sec {
                0x00 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("sll"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rtn),
                        DisasmToken::Literal(format!("{}", shamt)),
                    ]);
                }
                0x02 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("srl"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rtn),
                        DisasmToken::Literal(format!("{}", shamt)),
                    ]);
                }
                0x03 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("sra"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rtn),
                        DisasmToken::Literal(format!("{}", shamt)),
                    ]);
                }
                0x04 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("sllv"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rtn),
                        DisasmToken::RegisterName(rsn),
                    ]);
                }
                0x06 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("srlv"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rtn),
                        DisasmToken::RegisterName(rsn),
                    ]);
                }
                0x07 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("srav"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rtn),
                        DisasmToken::RegisterName(rsn),
                    ]);
                }
                0x08 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("jr"),
                        DisasmToken::RegisterName(rsn),
                    ]);
                }
                0x09 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("jalr"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                    ]);
                }
                0x0C => {
                    line.push_tokens(vec![DisasmToken::Mnemonic("syscall")]);
                }
                0x0D => {
                    line.push_tokens(vec![DisasmToken::Mnemonic("break")]);
                }
                0x10 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("mfhi"),
                        DisasmToken::RegisterName(rdn),
                    ]);
                }
                0x11 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("mthi"),
                        DisasmToken::RegisterName(rsn),
                    ]);
                }
                0x12 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("mflo"),
                        DisasmToken::RegisterName(rdn),
                    ]);
                }
                0x13 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("mtlo"),
                        DisasmToken::RegisterName(rsn),
                    ]);
                }
                0x18 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("mult"),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x19 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("multu"),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x1A => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("div"),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x1B => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("divu"),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x20 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("add"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x21 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("addu"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x22 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("sub"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x23 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("subu"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x24 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("and"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x25 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("or"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x26 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("xor"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x27 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("nor"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x2A => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("slt"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                0x2B => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("sltu"),
                        DisasmToken::RegisterName(rdn),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::RegisterName(rtn),
                    ]);
                }
                _ => {
                    line.push_tokens(vec![DisasmToken::Mnemonic("UNKNOWN")]);
                }
            }
        }
        0x01 => {
            // REGIMM/BcondZ: rt field distinguishes
            match rt {
                0x00 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("bltz"),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::Literal(format!("{:#010x}", branch_target)),
                    ]);
                }
                0x01 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("bgez"),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::Literal(format!("{:#010x}", branch_target)),
                    ]);
                }
                0x10 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("bltzal"),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::Literal(format!("{:#010x}", branch_target)),
                    ]);
                }
                0x11 => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("bgezal"),
                        DisasmToken::RegisterName(rsn),
                        DisasmToken::Literal(format!("{:#010x}", branch_target)),
                    ]);
                }
                _ => {
                    line.push_tokens(vec![
                        DisasmToken::Mnemonic("unknown"),
                        DisasmToken::Literal(format!("{:#010x}", instr)),
                    ]);
                }
            }
        }
        0x02 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("j"),
                DisasmToken::Literal(format!("{:#010x}", target_addr)),
            ]);
        }
        0x03 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("jal"),
                DisasmToken::Literal(format!("{:#010x}", target_addr)),
            ]);
        }
        0x04 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("beq"),
                DisasmToken::RegisterName(rsn),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{:#010x}", branch_target)),
            ]);
        }
        0x05 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("bne"),
                DisasmToken::RegisterName(rsn),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{:#010x}", branch_target)),
            ]);
        }
        0x06 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("blez"),
                DisasmToken::RegisterName(rsn),
                DisasmToken::Literal(format!("{:#010x}", branch_target)),
            ]);
        }
        0x07 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("bgtz"),
                DisasmToken::RegisterName(rsn),
                DisasmToken::Literal(format!("{:#010x}", branch_target)),
            ]);
        }
        0x08 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("addi"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::RegisterName(rsn),
                DisasmToken::Literal(format!("{}", simm)),
            ]);
        }
        0x09 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("addiu"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::RegisterName(rsn),
                DisasmToken::Literal(format!("{}", simm)),
            ]);
        }
        0x0A => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("slti"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::RegisterName(rsn),
                DisasmToken::Literal(format!("{}", simm)),
            ]);
        }
        0x0B => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("sltiu"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::RegisterName(rsn),
                DisasmToken::Literal(format!("{}", simm)),
            ]);
        }
        0x0C => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("andi"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::RegisterName(rsn),
                DisasmToken::Literal(format!("{:#x}", imm)),
            ]);
        }
        0x0D => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("ori"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::RegisterName(rsn),
                DisasmToken::Literal(format!("{:#x}", imm)),
            ]);
        }
        0x0E => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("xori"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::RegisterName(rsn),
                DisasmToken::Literal(format!("{:#x}", imm)),
            ]);
        }
        0x0F => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("lui"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{:#x}", imm)),
            ]);
        }
        0x10 => {
            // COP0
            if (instr & (1 << 25)) != 0 {
                // CO bit set - coprocessor operation
                let cop_funct = sec;
                match cop_funct {
                    0x10 => {
                        line.push_tokens(vec![DisasmToken::Mnemonic("rfe")]);
                    }
                    _ => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("cop0"),
                            DisasmToken::Literal(format!("{:#x}", instr & 0x01FFFFFF)),
                        ]);
                    }
                }
            } else {
                // MFC0/CFC0/MTC0/CTC0
                match rs {
                    0x00 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("mfc0"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x02 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("cfc0"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x04 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("mtc0"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x06 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("ctc0"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x08 => {
                        // BCnF/BCnT
                        if (rt & 0x01) == 0 {
                            line.push_tokens(vec![
                                DisasmToken::Mnemonic("bc0f"),
                                DisasmToken::Literal(format!("{:#010x}", branch_target)),
                            ]);
                        } else {
                            line.push_tokens(vec![
                                DisasmToken::Mnemonic("bc0t"),
                                DisasmToken::Literal(format!("{:#010x}", branch_target)),
                            ]);
                        }
                    }
                    _ => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("cop0"),
                            DisasmToken::Literal(format!("{:#010x}", instr)),
                        ]);
                    }
                }
            }
        }
        0x11 => {
            // COP1
            if (instr & (1 << 25)) != 0 {
                line.push_tokens(vec![
                    DisasmToken::Mnemonic("cop1"),
                    DisasmToken::Literal(format!("{:#x}", instr & 0x01FFFFFF)),
                ]);
            } else {
                match rs {
                    0x00 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("mfc1"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x02 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("cfc1"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x04 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("mtc1"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x06 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("ctc1"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x08 => {
                        if (rt & 0x01) == 0 {
                            line.push_tokens(vec![
                                DisasmToken::Mnemonic("bc1f"),
                                DisasmToken::Literal(format!("{:#010x}", branch_target)),
                            ]);
                        } else {
                            line.push_tokens(vec![
                                DisasmToken::Mnemonic("bc1t"),
                                DisasmToken::Literal(format!("{:#010x}", branch_target)),
                            ]);
                        }
                    }
                    _ => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("cop1"),
                            DisasmToken::Literal(format!("{:#010x}", instr)),
                        ]);
                    }
                }
            }
        }
        0x12 => {
            // COP2 (GTE on PSX)
            if (instr & (1 << 25)) != 0 {
                line.push_tokens(vec![
                    DisasmToken::Mnemonic("cop2"),
                    DisasmToken::Literal(format!("{:#x}", instr & 0x01FFFFFF)),
                ]);
            } else {
                match rs {
                    0x00 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("mfc2"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x02 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("cfc2"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x04 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("mtc2"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x06 => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("ctc2"),
                            DisasmToken::RegisterName(rtn),
                            DisasmToken::RegisterName(rdn),
                        ]);
                    }
                    0x08 => {
                        if (rt & 0x01) == 0 {
                            line.push_tokens(vec![
                                DisasmToken::Mnemonic("bc2f"),
                                DisasmToken::Literal(format!("{:#010x}", branch_target)),
                            ]);
                        } else {
                            line.push_tokens(vec![
                                DisasmToken::Mnemonic("bc2t"),
                                DisasmToken::Literal(format!("{:#010x}", branch_target)),
                            ]);
                        }
                    }
                    _ => {
                        line.push_tokens(vec![
                            DisasmToken::Mnemonic("cop2"),
                            DisasmToken::Literal(format!("{:#010x}", instr)),
                        ]);
                    }
                }
            }
        }
        0x13 | 0x30 | 0x31 | 0x33 | 0x38 | 0x39 | 0x3B => {
            // COP3, LWC0, LWC1, LWC3, SWC0, SWC1, SWC3 are not used in PS1
            line.push_tokens(vec![DisasmToken::Mnemonic("UNKNOWN")]);
        }
        // Loads
        0x20 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("lb"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        0x21 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("lh"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        0x22 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("lwl"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        0x23 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("lw"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        0x24 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("lbu"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        0x25 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("lhu"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        0x26 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("lwr"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        // Stores
        0x28 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("sb"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        0x29 => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("sh"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        0x2A => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("swl"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        0x2B => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("sw"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        0x2E => {
            line.push_tokens(vec![
                DisasmToken::Mnemonic("swr"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        // Coprocessor loads
        0x32 => {
            // LWC2 (GTE)
            line.push_tokens(vec![
                DisasmToken::Mnemonic("lwc2"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        // Coprocessor stores
        0x3A => {
            // SWC2 (GTE)
            line.push_tokens(vec![
                DisasmToken::Mnemonic("swc2"),
                DisasmToken::RegisterName(rtn),
                DisasmToken::Literal(format!("{}({})", simm, rsn)),
            ]);
        }
        _ => {
            line.push_tokens(vec![DisasmToken::Mnemonic("UNKNOWN")]);
        }
    }

    line
}
