use super::System;
use super::gamepad::Gamepad;
use super::memory_card::MemoryCard;

#[derive(Clone, Copy, PartialEq)]
enum State {
    None,
    GamepadComm,
    MemCardComm,
}

pub struct DeviceManager {
    current_state: State,
    pub gamepads: [Option<Gamepad>; 2],
    pub memcards: [Option<MemoryCard>; 2],
}

impl DeviceManager {
    pub const fn new(gamepads: [Option<Gamepad>; 2], memcards: [Option<MemoryCard>; 2]) -> Self {
        Self {
            gamepads,
            memcards,
            current_state: State::None,
        }
    }

    pub fn reset(&mut self) {
        if self.current_state == State::None {
            return;
        }

        self.gamepads.iter_mut().flatten().for_each(Gamepad::reset);
        self.memcards
            .iter_mut()
            .flatten()
            .for_each(MemoryCard::reset);
        self.current_state = State::None;
    }

    pub fn send_and_receive_byte(system: &mut System, data: u8) -> (u8, bool) {
        let sio = &mut system.sio0;
        let port = usize::from(sio.control.port_select());

        let (byte, next_state) = match sio.device_manager.current_state {
            State::None => match data {
                0x01 => sio.device_manager.process_gamepad_communication(port, data),
                0x81 => sio.device_manager.process_memcard_communication(port, data),
                _ => (0xFF, State::None), // memory card and other peripherals not connected
            },
            State::GamepadComm => sio.device_manager.process_gamepad_communication(port, data),
            State::MemCardComm => sio.device_manager.process_memcard_communication(port, data),
        };

        sio.device_manager.current_state = next_state;
        (byte, next_state != State::None)
    }

    fn process_memcard_communication(&mut self, port: usize, data: u8) -> (u8, State) {
        self.memcards[port]
            .as_mut()
            .map_or((0xFF, State::None), |mc| {
                let byte = mc.send_and_receive_byte(data);
                let state = if mc.in_ack() {
                    State::MemCardComm
                } else {
                    State::None
                };
                (byte, state)
            })
    }

    fn process_gamepad_communication(&mut self, port: usize, data: u8) -> (u8, State) {
        self.gamepads[port]
            .as_mut()
            .map_or((0xFF, State::None), |gp| {
                let byte = gp.send_and_receive_byte(data);
                let state = if gp.in_ack() {
                    State::GamepadComm
                } else {
                    State::None
                };
                (byte, state)
            })
    }
}
