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

#[derive(Default)]
pub struct Gamepad {
    state: GamepadState,
}

impl Gamepad {
    pub fn send_and_receive_byte(system: &mut System, _data: u8) {
        let gamepad = &mut system.sio.gamepad;
        let received = match gamepad.state {
            GamepadState::Init => 0xFF,

            // Gamepad ID: 0x5A41 -> Digital Pad
            GamepadState::IdLow => 0x41,
            GamepadState::IdHigh => 0x5A,

            // Gamepad switches state
            GamepadState::SwitchLow => 0xFF,
            GamepadState::SwitchHigh => 0xFF,

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
        }
    }

    pub fn reset(&mut self) {
        self.state = GamepadState::Init;
    }
}
