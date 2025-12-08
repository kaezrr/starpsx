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
    AnalogInput0,
    AnalogInput1,
    AnalogInput2,
    AnalogInput3,
}

impl GamepadState {
    fn next(self) -> Self {
        let idx = self as usize;
        GAMEPAD_STATES[(idx + 1) % GAMEPAD_STATES.len()]
    }
}

const GAMEPAD_STATES: [GamepadState; 9] = [
    GamepadState::Init,
    GamepadState::IdLow,
    GamepadState::IdHigh,
    GamepadState::SwitchLow,
    GamepadState::SwitchHigh,
    GamepadState::AnalogInput0,
    GamepadState::AnalogInput1,
    GamepadState::AnalogInput2,
    GamepadState::AnalogInput3,
];

#[repr(C)]
pub enum Button {
    Select,
    L3,
    R3,
    Start,
    Up,
    Down,
    Left,
    Right,
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
}

impl Gamepad {
    pub fn send_and_receive_byte(system: &mut System, data: u8) {
        let gamepad = &mut system.sio.gamepad;

        // Check for valid communication sequence
        match gamepad.state {
            GamepadState::Init if data != 0x01 => panic!("Wrong controller init command"),
            GamepadState::IdLow if data != b'B' => panic!("Wrong controller read id low command"),
            _ => (),
        }

        let received = match gamepad.state {
            GamepadState::Init => 0xFF,

            // Gamepad ID: 0x5A41 -> Digital Pad
            GamepadState::IdLow => 0x41,
            GamepadState::IdHigh => 0x5A,

            // Gamepad switches state
            GamepadState::SwitchLow => gamepad.switch_byte() as u8,
            GamepadState::SwitchHigh => (gamepad.switch_byte() >> 8) as u8,

            // Gamepad analog stick state
            GamepadState::AnalogInput0 => todo!("Analog stick 0"),
            GamepadState::AnalogInput1 => todo!("Analog stick 1"),
            GamepadState::AnalogInput2 => todo!("Analog stick 2"),
            GamepadState::AnalogInput3 => todo!("Analog stick 3"),
        };

        gamepad.state = gamepad.state.next();
        system.sio.push_received_data(received);

        // Controller and Memory Card received byte interrupt
        if system.sio.control.dsr_interrupt_enable() {
            system.irqctl.stat().set_ctl_mem(true);
            system.sio.status.set_irq(true);
        }
    }

    pub fn reset(&mut self) {
        self.state = GamepadState::Init;
    }

    pub fn set_button_state(&mut self, button: Button, pressed: bool) {
        self.digital_switches[button as usize] = pressed;
    }

    fn switch_byte(&self) -> u16 {
        let mut v = 0u16;
        for i in 0..16 {
            v |= (!self.digital_switches[i] as u16) << i;
        }
        v
    }
}
