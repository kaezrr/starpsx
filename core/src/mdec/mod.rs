mod util;

use std::collections::VecDeque;

use num_enum::FromPrimitive;

use crate::System;
use crate::mdec::util::ZAG_ZIG;
use crate::mdec::util::level_shift_4bpp;
use crate::mdec::util::level_shift_8bpp;
use crate::mdec::util::signed10bit;
use crate::mdec::util::yuv_to_rgb15_block;
use crate::mdec::util::yuv_to_rgb24_block;
use crate::mem::ByteAddressable;

pub const PADDR_START: u32 = 0x1F801820;
pub const PADDR_END: u32 = 0x1F801828;

bitfield::bitfield! {
    #[derive(Default)]
    struct Status(u32);
    _, set_out_fifo_empty: 31;
    _, set_in_fifo_full: 30;
    _, set_busy: 29;
    _, set_data_in: 28;
    _, set_data_out: 27;
    _, set_cmd_data_out: 26, 23;
}

bitfield::bitfield! {
    struct CommandWord(u32);
    command, _: 31, 29;
    data_out, _: 28, 25;

    into Depth, output_depth, _: 28, 27;
    output_signed, _: 26;
    output_bit15, _: 25;
    u16, params_len, _: 15, 0;

    into Color, color, _: 0, 0;
}

#[derive(Debug, FromPrimitive, Clone, Copy)]
#[repr(u32)]
enum Depth {
    #[default]
    Bit4 = 0,
    Bit8 = 1,
    Bit24 = 2,
    Bit15 = 3,
}

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq)]
#[repr(u32)]
enum Color {
    #[default]
    Luminance = 0,
    LuminanceAndColor = 1,
}

#[derive(Debug, Clone, Copy)]
enum CommandType {
    SetQuantTable(Color),
    SetScaleTable,
    DecodeMacroblock {
        depth: Depth,
        is_signed: bool,
        b15: bool,
    },
}

struct Command {
    command_type: CommandType,
    parameters: Vec<u32>,
}

pub struct MacroDecoder {
    status: Status,
    collecting: Option<Command>,

    output_fifo: VecDeque<u32>, // 32 words
    params_remaining: u16,

    scale_table: [i16; 64],
    luminance_table: [u8; 64],
    chrominance_table: [u8; 64],
}

impl Default for MacroDecoder {
    fn default() -> Self {
        Self {
            status: Status::default(),
            collecting: None,
            output_fifo: VecDeque::default(),
            params_remaining: 0,
            scale_table: [0; 64],
            luminance_table: [0; 64],
            chrominance_table: [0; 64],
        }
    }
}
impl MacroDecoder {
    fn status(&self) -> u32 {
        // Bits 0-15 show number of remaining parameters minus 1, 0xFFFF = 0
        (self.status.0 & !0xFFFF) | self.params_remaining.wrapping_sub(1) as u32
    }

    fn write_control(&mut self, data: u32) {
        // Reset
        if data & (1 << 31) != 0 {
            self.status.0 = 0x80040000;
            self.params_remaining = 0;
            self.collecting = None;
        }

        self.status.set_data_in(data & (1 << 30) != 0);
        self.status.set_data_out(data & (1 << 29) != 0);
    }

    fn decode_command(&mut self, data: u32) {
        let cmd = CommandWord(data);
        // Status bits 26-23 reflect command bits 28-25
        self.status.set_cmd_data_out(cmd.data_out());

        match cmd.command() {
            // No Function
            0 | 4..8 => {
                // Doesn't have the "minus 1" effect
                self.params_remaining = cmd.params_len() + 1;
            }

            1 => {
                let depth = cmd.output_depth();
                let is_signed = cmd.output_signed();
                let b15_set = cmd.output_bit15();

                self.status.set_busy(true);
                self.params_remaining = cmd.params_len();
                self.collecting = Some(Command {
                    command_type: CommandType::DecodeMacroblock {
                        depth,
                        is_signed,
                        b15: b15_set,
                    },
                    parameters: vec![],
                });
            }

            2 => {
                let color = cmd.color();

                self.status.set_busy(true);
                self.params_remaining = match color {
                    Color::Luminance => 16, // 64 unsigned bytes
                    Color::LuminanceAndColor => 32,
                };
                self.collecting = Some(Command {
                    command_type: CommandType::SetQuantTable(color),
                    parameters: vec![],
                });
            }

            3 => {
                self.status.set_busy(true);
                self.params_remaining = 32; // 64 signed halfwords
                self.collecting = Some(Command {
                    command_type: CommandType::SetScaleTable,
                    parameters: vec![],
                });
            }

            _ => unreachable!("3 bit value"),
        }
    }

    fn handle_command(&mut self, collected: Command) {
        match collected.command_type {
            CommandType::DecodeMacroblock {
                depth,
                is_signed,
                b15,
            } => {
                let raw: &[u16] = bytemuck::cast_slice(collected.parameters.as_slice());
                let mut source: VecDeque<u16> = raw.iter().copied().collect();

                match depth {
                    Depth::Bit4 => {
                        let block = self.decode_block(&mut source, &self.luminance_table);
                        let pixels = level_shift_4bpp(block);

                        let words: [u32; 8] = bytemuck::cast(pixels);
                        self.output_fifo.extend(words);
                    }

                    Depth::Bit8 => {
                        let block = self.decode_block(&mut source, &self.luminance_table);
                        let pixels = level_shift_8bpp(block);

                        let words: [u32; 16] = bytemuck::cast(pixels);
                        self.output_fifo.extend(words);
                    }

                    Depth::Bit15 => {
                        while !source.is_empty() {
                            // Skip any trailing padding before checking if there's real data left
                            while source.front() == Some(&0xFE00) {
                                source.pop_front();
                            }

                            if source.is_empty() {
                                break;
                            }

                            let cr = self.decode_block(&mut source, &self.chrominance_table);
                            let cb = self.decode_block(&mut source, &self.chrominance_table);

                            let mut dst = [0u16; 256];

                            let y1 = self.decode_block(&mut source, &self.luminance_table);
                            yuv_to_rgb15_block(&cr, &cb, &y1, (0, 0), is_signed, b15, &mut dst);

                            let y2 = self.decode_block(&mut source, &self.luminance_table);
                            yuv_to_rgb15_block(&cr, &cb, &y2, (8, 0), is_signed, b15, &mut dst);

                            let y3 = self.decode_block(&mut source, &self.luminance_table);
                            yuv_to_rgb15_block(&cr, &cb, &y3, (0, 8), is_signed, b15, &mut dst);

                            let y4 = self.decode_block(&mut source, &self.luminance_table);
                            yuv_to_rgb15_block(&cr, &cb, &y4, (8, 8), is_signed, b15, &mut dst);

                            let words: &[u32] = bytemuck::cast_slice(&dst);
                            self.output_fifo.extend(words);
                        }
                    }

                    Depth::Bit24 => {
                        while !source.is_empty() {
                            // Skip any trailing padding before checking if there's real data left
                            while source.front() == Some(&0xFE00) {
                                source.pop_front();
                            }

                            if source.is_empty() {
                                break;
                            }

                            let cr = self.decode_block(&mut source, &self.chrominance_table);
                            let cb = self.decode_block(&mut source, &self.chrominance_table);

                            let mut dst = [0u8; 768];

                            let y1 = self.decode_block(&mut source, &self.luminance_table);
                            yuv_to_rgb24_block(&cr, &cb, &y1, (0, 0), is_signed, &mut dst);

                            let y2 = self.decode_block(&mut source, &self.luminance_table);
                            yuv_to_rgb24_block(&cr, &cb, &y2, (8, 0), is_signed, &mut dst);

                            let y3 = self.decode_block(&mut source, &self.luminance_table);
                            yuv_to_rgb24_block(&cr, &cb, &y3, (0, 8), is_signed, &mut dst);

                            let y4 = self.decode_block(&mut source, &self.luminance_table);
                            yuv_to_rgb24_block(&cr, &cb, &y4, (8, 8), is_signed, &mut dst);

                            let words: &[u32] = bytemuck::cast_slice(&dst);
                            self.output_fifo.extend(words);
                        }
                    }
                }

                self.status.set_out_fifo_empty(false);
            }

            CommandType::SetQuantTable(color) => {
                let raw_bytes: &[u8] = bytemuck::cast_slice(collected.parameters.as_slice());
                self.luminance_table.copy_from_slice(&raw_bytes[0..64]);
                if color == Color::LuminanceAndColor {
                    self.chrominance_table.copy_from_slice(&raw_bytes[64..128]);
                }
            }

            CommandType::SetScaleTable => {
                let scale_table: [u32; 32] = collected.parameters.clone().try_into().unwrap();
                self.scale_table = bytemuck::cast(scale_table);
            }
        }
    }

    pub fn command_or_param(&mut self, data: u32) {
        let collecting_command = self.collecting.take();
        match collecting_command {
            None => self.decode_command(data),
            Some(mut collected) => {
                self.params_remaining -= 1;
                collected.parameters.push(data);

                if self.params_remaining == 0 {
                    self.status.set_busy(false);
                    self.handle_command(collected); // Consume
                } else {
                    self.collecting = Some(collected);
                }
            }
        }
    }

    pub fn response(&mut self) -> u32 {
        let data = self.output_fifo.pop_front().unwrap_or(0xFE00FE00);
        if self.output_fifo.is_empty() {
            self.status.set_out_fifo_empty(true);
        }
        data
    }

    fn inverse_discrete_cosine(&self, src: &mut [i16; 64]) {
        let dst = &mut [0i16; 64];

        for _ in 0..2 {
            for x in 0..8 {
                for y in 0..8 {
                    let mut sum: i32 = 0;
                    for z in 0..8 {
                        sum += src[y + z * 8] as i32 * (self.scale_table[x + z * 8] as i32 / 8);
                    }
                    dst[x + y * 8] = ((sum + 0xFFF) / 0x2000) as i16;
                }
            }
            std::mem::swap(src, dst);
        }
    }

    pub fn decode_block(&self, source: &mut VecDeque<u16>, qt: &[u8]) -> [i16; 64] {
        let mut block = [0; 64];
        let mut k: usize = 0;

        while source.front() == Some(&0xFE00) {
            source.pop_front();
        }
        if source.is_empty() {
            return [0; 64];
        }

        let first_word = source.pop_front().unwrap();
        let q_fact = ((first_word >> 10) & 0x3F) as i32;
        let dc_coeff = signed10bit(first_word & 0x3FF);
        let dc_val = if q_fact == 0 {
            (dc_coeff * 2).clamp(-0x400, 0x3FF)
        } else {
            (dc_coeff * qt[0] as i16).clamp(-0x400, 0x3FF)
        };
        block[ZAG_ZIG[0]] = dc_val;

        while let Some(&n) = source.front() {
            if n == 0xFE00 {
                break;
            }
            source.pop_front();

            k += 1 + ((n >> 10) & 0x3F) as usize;
            if k > 63 {
                break;
            }
            let ac_level = signed10bit(n & 0x3FF) as i32;
            let val = if q_fact == 0 {
                (ac_level * 2).clamp(-0x400, 0x3FF)
            } else {
                let qt_val = qt[k] as i32;
                ((ac_level * qt_val * q_fact + 4) / 8).clamp(-0x400, 0x3FF)
            };
            let target_idx = if q_fact == 0 { k } else { ZAG_ZIG[k] };
            block[target_idx] = val as i16;

            if k == 63 {
                break; // block is full, don't consume the next word
            }
        }

        self.inverse_discrete_cosine(&mut block);
        block
    }
}

pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> T {
    let data = match addr {
        0x1F801820 => system.mdec.response(),
        0x1F801824 => system.mdec.status(),
        _ => unimplemented!("MDEC read {addr:x}"),
    };

    T::from_u32(data)
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, data: T) {
    let data = data.to_u32();

    match addr {
        0x1F801820 => system.mdec.command_or_param(data),
        0x1F801824 => system.mdec.write_control(data),
        _ => unimplemented!("MDEC write {addr:x}={data:x}"),
    }
}
