use arrayvec::ArrayVec;
use num_enum::FromPrimitive;
use tracing::debug;

use crate::System;
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

#[derive(Debug, Clone, Copy, FromPrimitive)]
#[repr(u32)]
enum Color {
    #[default]
    Luminance = 0,
    LuminanceAndColor = 1,
}

#[derive(Debug, Clone, Copy)]
enum CommandType {
    DecodeMacroblock(Depth, bool, bool),
    SetQuantTable(Color),
    SetScaleTable,
}

struct Command {
    command_type: CommandType,
    parameters: ArrayVec<u32, 32>,
}

pub struct MacroDecoder {
    status: Status,
    collecting: Option<Command>,

    output_fifo: ArrayVec<u32, 32>, // 32 words
    params_remaining: u16,

    scale_table: [i16; 64],
}

impl Default for MacroDecoder {
    fn default() -> Self {
        Self {
            status: Status::default(),
            collecting: None,
            output_fifo: ArrayVec::default(),
            params_remaining: 0,
            scale_table: [0; 64],
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
                debug!("no function");
                // Doesn't have the "minus 1" effect
                self.params_remaining = cmd.params_len() + 1;
            }

            1 => {
                let output_depth = cmd.output_depth();
                let is_signed = cmd.output_signed();
                let is_bit15_set = cmd.output_bit15();

                debug!(
                    ?output_depth,
                    is_signed,
                    is_bit15_set,
                    len = cmd.params_len(),
                    "decode macroblock"
                );

                self.status.set_busy(true);
                self.params_remaining = cmd.params_len();
                self.collecting = Some(Command {
                    command_type: CommandType::DecodeMacroblock(
                        output_depth,
                        is_signed,
                        is_bit15_set,
                    ),
                    parameters: ArrayVec::default(),
                });
            }

            2 => {
                let color = cmd.color();
                debug!(?color, "set quant table with");

                self.status.set_busy(true);
                self.params_remaining = match color {
                    Color::Luminance => 16, // 64 unsigned bytes
                    Color::LuminanceAndColor => 32,
                };
                self.collecting = Some(Command {
                    command_type: CommandType::SetQuantTable(color),
                    parameters: ArrayVec::default(),
                });
            }

            3 => {
                debug!("set scale table");
                self.status.set_busy(true);
                self.params_remaining = 32; // 64 signed halfwords
                self.collecting = Some(Command {
                    command_type: CommandType::SetScaleTable,
                    parameters: ArrayVec::default(),
                });
            }

            _ => unreachable!("3 bit value"),
        }
    }

    fn handle_command(&mut self, collected: Command) {
        match collected.command_type {
            CommandType::DecodeMacroblock(depth, is_signed, is_bit15_set) => {
                debug!("handle command decode macroblock, {depth:?}, {is_signed}, {is_bit15_set}");
                for _ in 0..32 {
                    self.output_fifo.push(0xF8C8DC);
                }
                self.status.set_out_fifo_empty(false);
            }
            CommandType::SetQuantTable(color) => debug!("handle set quant table, {color:?}"),
            CommandType::SetScaleTable => {
                self.scale_table = bytemuck::cast(collected.parameters.into_inner().unwrap());
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
        let data = self.output_fifo.pop().unwrap_or(0xDEADBEEF);
        if self.output_fifo.is_empty() {
            self.status.set_out_fifo_empty(true);
        }
        data
    }
}

pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> T {
    let data = match addr {
        0x1F801820 => system.mdec.response(),
        0x1F801824 => system.mdec.status(),
        _ => unimplemented!("MDEC read {addr:x}"),
    };

    debug!("mdec read {addr:x}={data:x}");

    T::from_u32(data)
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, data: T) {
    let data = data.to_u32();

    debug!("mdec write {addr:x}={data:x}");

    match addr {
        0x1F801820 => system.mdec.command_or_param(data),
        0x1F801824 => system.mdec.write_control(data),
        _ => unimplemented!("MDEC write {addr:x}={data:x}"),
    }
}
