use tracing::{debug, trace, warn};

const FRAME_SIZE: usize = 0x80;

pub struct MemoryCard {
    in_ack: bool,
    state: State,
    command: Command,

    state_idx: usize,
    sector_number: u16,
    checksum: u8,

    sector: [u8; 128],
    bytes_left: usize,

    end_response: EndResponse,
    directory_not_read: bool,

    format: Box<[u8; 0x20000]>,
}

impl Default for MemoryCard {
    fn default() -> Self {
        Self {
            in_ack: false,
            state: State::Init,
            command: Command::Read,

            state_idx: 0,
            sector_number: 0,
            checksum: 0,

            sector: [0; 128],
            bytes_left: 0,

            end_response: EndResponse::Good,
            directory_not_read: true,

            format: Box::new(*include_bytes!("blank.mcd")),
        }
    }
}

impl MemoryCard {
    pub fn send_and_receive_byte(&mut self, data: u8) -> u8 {
        let send = match self.state {
            State::Init => 0xFF,
            State::CardId1 => 0x5A,
            State::CardId2 => 0x5D,
            State::CmdAck1 => 0x5C,
            State::CmdAck2 => 0x5D,
            State::Recv04h => 0x04,
            State::Recv00h => 0x00,
            State::Recv80h => 0x80,
            State::AckMsb => (self.sector_number >> 8) as u8,
            State::AckLsb => (self.sector_number & 0xFF) as u8,
            State::Flag => (self.directory_not_read as u8) << 3,

            State::SendMsb => {
                self.sector_number = u16::from(data) << 8;
                self.checksum = data;
                0x00
            }

            State::SendLsb => {
                self.sector_number |= u16::from(data);
                self.checksum ^= data;

                if self.sector_number > 0x3FF {
                    warn!(
                        "invalid sector address={:#x}, aborting transfer",
                        self.sector_number
                    );
                    self.sector_number = 0xFFFF;
                    self.end_response = EndResponse::BadSector;
                }

                0x00
            }

            State::RecvSector => {
                if data != 0 {
                    warn!("RecvSector: unexpected byte from host: {data:#x}");
                }

                let byte = self.sector[128 - self.bytes_left];
                self.bytes_left -= 1;
                self.checksum ^= byte;

                if self.bytes_left > 0 {
                    return byte;
                }

                byte
            }

            State::SendSector => {
                self.sector[128 - self.bytes_left] = data;
                self.bytes_left -= 1;
                self.checksum ^= data;

                if self.bytes_left > 0 {
                    return 0;
                }

                0
            }

            State::RecvChecksum => self.checksum,
            State::SendChecksum => {
                self.end_response = if self.checksum == data {
                    EndResponse::Good
                } else {
                    EndResponse::BadChecksum
                };
                0
            }

            State::MemEnd => {
                if self.command == Command::Write {
                    if matches!(self.end_response, EndResponse::Good) {
                        self.save_sector();
                    }
                    self.directory_not_read = false;
                }

                debug!(cmd=?self.command, sector=self.sector_number, "DONE!");
                self.end_response as u8
            }
        };

        trace!(state=?self.state, command=?self.command, "memcard recv={data:#x} send={send:#x}");

        if let Some((next_state, next_idx)) = self.command.next(self.state, data, self.state_idx) {
            if matches!(next_state, State::RecvSector) {
                self.load_sector();
                self.bytes_left = 128;
            }

            if matches!(next_state, State::SendSector) {
                self.bytes_left = 128;
            }

            self.state = next_state;
            self.state_idx = next_idx;
            self.in_ack = self.state != State::Init;

            send
        } else {
            self.reset();
            0xFF
        }
    }

    pub fn in_ack(&self) -> bool {
        self.in_ack
    }

    pub fn reset(&mut self) {
        self.in_ack = false;
        self.state = State::Init;
        self.state_idx = 0;
    }

    pub fn load_sector(&mut self) {
        let address = (self.sector_number as usize) * FRAME_SIZE;
        self.sector
            .copy_from_slice(&self.format[address..address + 128]);
    }

    pub fn save_sector(&mut self) {
        let address = (self.sector_number as usize) * FRAME_SIZE;
        self.format[address..address + 128].copy_from_slice(&self.sector);
    }
}

#[derive(Default, Clone, Copy)]
#[repr(u8)]
enum EndResponse {
    #[default]
    Good = 0x47,
    BadChecksum = 0x4E,
    BadSector = 0xFF,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum State {
    #[default]
    Init,
    Flag,
    CardId1,
    CardId2,
    CmdAck1,
    CmdAck2,
    Recv04h,
    Recv00h,
    Recv80h,
    SendMsb,
    SendLsb,
    SendSector,
    RecvSector,
    SendChecksum,
    RecvChecksum,
    AckMsb,
    AckLsb,
    MemEnd,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum Command {
    #[default]
    Read,
    Write,
    GetId,
}

impl Command {
    const GETID_STATES: [(State, Option<u8>); 10] = [
        (State::Init, None),
        (State::Flag, Some(0x81)),
        (State::CardId1, Some(0x53)),
        (State::CardId2, Some(0x00)),
        (State::CmdAck1, Some(0x00)),
        (State::CmdAck2, Some(0x00)),
        (State::Recv04h, Some(0x00)),
        (State::Recv00h, Some(0x00)),
        (State::Recv00h, Some(0x00)),
        (State::Recv80h, Some(0x00)),
    ];

    const READ_STATES: [(State, Option<u8>); 13] = [
        (State::Init, None),
        (State::Flag, Some(0x81)),
        (State::CardId1, Some(0x52)),
        (State::CardId2, Some(0x00)),
        (State::SendMsb, Some(0x00)),
        (State::SendLsb, None),
        (State::CmdAck1, None),
        (State::CmdAck2, Some(0x00)),
        (State::AckMsb, Some(0x00)),
        (State::AckLsb, Some(0x00)),
        (State::RecvSector, Some(0x00)),   // 128 bytes
        (State::RecvChecksum, Some(0x00)), // MSB xor LSB xor Data bytes
        (State::MemEnd, Some(0x00)),
    ];

    const WRITE_STATES: [(State, Option<u8>); 11] = [
        (State::Init, None),
        (State::Flag, Some(0x81)),
        (State::CardId1, Some(0x57)),
        (State::CardId2, Some(0x00)),
        (State::SendMsb, Some(0x00)),
        (State::SendLsb, None),
        (State::SendSector, None),   // 128 bytes
        (State::SendChecksum, None), // MSB xor LSB xor Data bytes
        (State::CmdAck1, None),
        (State::CmdAck2, Some(0x00)),
        (State::MemEnd, Some(0x00)),
    ];

    fn states_table(&self) -> &'static [(State, Option<u8>)] {
        match self {
            Command::Read => &Command::READ_STATES,
            Command::Write => &Command::WRITE_STATES,
            Command::GetId => &Command::GETID_STATES,
        }
    }

    pub fn next(&mut self, current: State, recv: u8, state_idx: usize) -> Option<(State, usize)> {
        if current == State::Flag {
            *self = match recv {
                0x52 => Command::Read,
                0x57 => Command::Write,
                0x53 => Command::GetId,
                _ => return None,
            };
        }

        let table = self.states_table();
        let next_idx = (state_idx + 1) % table.len();
        let (next_state, check_byte) = table[next_idx];

        let valid = check_byte.is_none_or(|b| b == recv);
        valid.then_some((next_state, next_idx))
    }
}
