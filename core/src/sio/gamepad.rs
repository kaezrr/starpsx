use crate::System;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
enum GamepadState {
    #[default]
    Init,
    IdLow,
    IdHigh,
    SwitchLow,
    SwitchHigh,
}

impl GamepadState {
    fn next(self, data: u8) -> Self {
        let idx = self as usize;
        let (next_state, valid_byte) = GAMEPAD_STATES[(idx + 1) % GAMEPAD_STATES.len()];

        if data != valid_byte {
            panic!("Invalid gamepad communication sequence");
        }

        next_state
    }
}

const GAMEPAD_STATES: [(GamepadState, u8); 5] = [
    (GamepadState::Init, 0x00),
    (GamepadState::IdLow, 0x01),
    (GamepadState::IdHigh, 0x42),
    (GamepadState::SwitchLow, 0x00),
    (GamepadState::SwitchHigh, 0x00),
];

#[repr(C)]
pub enum Button {
    Select,
    L3,
    R3,
    Start,
    Up,
    Right,
    Down,
    Left,
    L2,
    R2,
    L1,
    R1,
    Triangle,
    Circle,
    Cross,
    Square,
}

#[derive(Default)]
pub struct Gamepad {
    state: GamepadState,
    digital_switches: [bool; 16],
    in_ack: bool,
}

impl Gamepad {
    pub fn send_and_receive_byte(system: &mut System, data: u8) -> u8 {
        let gamepad = &mut system.sio.gamepad;

        let received = match gamepad.state {
            GamepadState::Init => 0xFF,

            // Gamepad ID: 0x5A41 -> Digital Pad
            GamepadState::IdLow => 0x41,
            GamepadState::IdHigh => 0x5A,

            // Gamepad switches state
            GamepadState::SwitchLow => gamepad.switch_halfbyte() as u8,
            GamepadState::SwitchHigh => (gamepad.switch_halfbyte() >> 8) as u8,
        };

        gamepad.state = gamepad.state.next(data);
        gamepad.in_ack = !matches!(gamepad.state, GamepadState::Init);

        received
    }

    pub fn reset(&mut self) {
        self.state = GamepadState::Init;
    }

    pub fn set_button_state(&mut self, button: Button, pressed: bool) {
        self.digital_switches[button as usize] = pressed;
    }

    pub fn in_ack(&self) -> bool {
        self.in_ack
    }

    fn switch_halfbyte(&self) -> u16 {
        let mut v = 0u16;
        for i in 0..16 {
            v |= (!self.digital_switches[i] as u16) << i;
        }
        v
    }
}
