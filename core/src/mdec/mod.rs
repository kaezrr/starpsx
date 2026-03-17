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

#[derive(Default)]
enum State {
    #[default]
    Command,
    Params(CommandType),
}

#[derive(Default)]
pub struct MacroDecoder {
    status: Status,
    state: State,
    params_len: u16,
    output_fifo: ArrayVec<u32, 32>, // 32 words
}

impl MacroDecoder {
    fn status(&self) -> u32 {
        // Bits 0-15 show number of remaining parameters minus 1, 0xFFFF = 0
        (self.status.0 & !0xFFFF) | self.params_len.wrapping_sub(1) as u32
    }

    fn write_control(&mut self, data: u32) {
        // Reset
        if data & (1 << 31) != 0 {
            self.status.0 = 0x80040000;
            self.params_len = 0;
            self.state = State::Command;
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
                self.params_len = cmd.params_len() + 1;
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

                self.params_len = cmd.params_len();
                self.status.set_busy(true);
                self.state = State::Params(CommandType::DecodeMacroblock(
                    output_depth,
                    is_signed,
                    is_bit15_set,
                ));
            }

            2 => {
                let color = cmd.color();
                debug!(?color, "set quant table with");

                self.state = State::Params(CommandType::SetQuantTable(color));
                self.status.set_busy(true);
                self.params_len = match color {
                    Color::Luminance => 16, // 64 unsigned bytes
                    Color::LuminanceAndColor => 32,
                };
            }

            3 => {
                debug!("set scale table");
                self.state = State::Params(CommandType::SetScaleTable);
                self.params_len = 32; // 64 signed halfwords
                self.status.set_busy(true);
            }

            _ => unreachable!("3 bit value"),
        }
    }

    fn handle_command(&mut self, command: CommandType) {
        match command {
            CommandType::DecodeMacroblock(depth, is_signed, is_bit15_set) => {
                debug!("handle command decode macroblock, {depth:?}, {is_signed}, {is_bit15_set}");
                for _ in 0..32 {
                    self.output_fifo.push(0xF8C8DC);
                }
                self.status.set_out_fifo_empty(false);
            }
            CommandType::SetQuantTable(color) => debug!("handle set quant table, {color:?}"),
            CommandType::SetScaleTable => debug!("handle set scale table"),
        }
    }

    pub fn command_or_param(&mut self, data: u32) {
        match self.state {
            State::Command => self.decode_command(data),
            State::Params(cmd) => {
                debug!("mdec param {data:x}");
                self.params_len -= 1;

                if self.params_len == 0 {
                    self.state = State::Command;
                    self.status.set_busy(false);
                    self.handle_command(cmd);
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
