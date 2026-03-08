use tracing::debug;

#[derive(Default)]
pub struct MemoryCard {
    in_ack: bool,
    state: State,
    command: Command,
    state_idx: usize,

    address: u16,
    checksum: u8,
    byte_count: u32,
}

impl MemoryCard {
    pub fn send_and_receive_byte(&mut self, data: u8) -> u8 {
        let send = match self.state {
            State::Init => 0xFF,
            State::Flag => 0x08,
            State::CardId1 => 0x5A,
            State::CardId2 => 0x5D,
            State::CmdAck1 => 0x5C,
            State::CmdAck2 => 0x5D,
            State::Recv04h => 0x04,
            State::Recv00h => 0x00,
            State::Recv80h => 0x80,
            State::AckMsb => (self.address >> 8) as u8,
            State::AckLsb => (self.address & 0xFF) as u8,

            State::SendMsb => {
                self.address |= u16::from(data) << 8;
                self.checksum = data;
                0x00
            }

            State::SendLsb => {
                self.address |= u16::from(data);
                self.checksum ^= data;
                0x00
            }

            State::RecvSector => {
                debug!("stubbed receive sector, sending 0");
                let byte = 0; // TODO: get this byte from a memcard file
                self.checksum ^= byte;
                byte
            }

            State::SendSector => todo!("memcard reply to sector data byte {data:#04x}"),

            State::RecvChecksum => self.checksum,
            State::SendChecksum => todo!("memcard reply to checksum byte {data:#04x}"),

            State::MemEnd => match self.command {
                Command::Read => 0x47,
                Command::Write => todo!("mem card write cmd end state"),
                Command::GetId => {
                    unreachable!("memory card GetId command doesn't have mem end state")
                }
            },
        };

        debug!(state=?self.state, command=?self.command, "memcard recv={data:x} send={send:x}");

        if self.byte_count > 0 {
            self.byte_count -= 1;
            return send;
        }

        if let Some((next_state, next_idx)) = self.command.next(self.state, data, self.state_idx) {
            if matches!(next_state, State::SendSector | State::RecvSector) {
                self.byte_count = 127; // need to send or receive 127 bytes after this one
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
        self.command = Command::GetId;
        self.state_idx = 0;
    }
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

#[derive(Debug, Default, Clone, Copy)]
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
