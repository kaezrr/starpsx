use crate::sio::memory_card::MemoryCard;

use super::*;

#[derive(Clone, Copy, PartialEq)]
enum State {
    None,
    GamepadComm,
    MemCardComm,
}

pub struct DeviceManager {
    gamepads: [Option<Gamepad>; 2],
    memcards: [Option<MemoryCard>; 2],
    current_state: State,
}

impl DeviceManager {
    pub fn new(gamepads: [Option<Gamepad>; 2], memcards: [Option<MemoryCard>; 2]) -> Self {
        Self {
            gamepads,
            memcards,
            current_state: State::None,
        }
    }

    pub fn reset(&mut self) {
        self.gamepads.iter_mut().flatten().for_each(|g| g.reset());
        self.current_state = State::None;
    }

    pub fn send_and_receive_byte(system: &mut System, data: u8) -> (u8, bool) {
        let sio = &mut system.sio0;
        let port = sio.control.port_select() as usize;

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

    pub fn gamepad_port_0_mut(&mut self) -> &mut Gamepad {
        self.gamepads[0].as_mut().unwrap()
    }

    fn process_memcard_communication(&mut self, port: usize, data: u8) -> (u8, State) {
        self.memcards[port]
            .as_mut()
            .map(|mc| {
                let byte = mc.send_and_receive_byte(data);
                let state = match mc.in_ack() {
                    true => State::MemCardComm,
                    false => State::None,
                };
                (byte, state)
            })
            .unwrap_or((0xFF, State::None))
    }

    fn process_gamepad_communication(&mut self, port: usize, data: u8) -> (u8, State) {
        self.gamepads[port]
            .as_mut()
            .map(|gp| {
                let byte = gp.send_and_receive_byte(data);
                let state = match gp.in_ack() {
                    true => State::GamepadComm,
                    false => State::None,
                };
                (byte, state)
            })
            .unwrap_or((0xFF, State::None))
    }
}
