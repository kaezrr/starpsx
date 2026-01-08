pub struct Gamepad {
    state: GamepadState,
    mode: GamepadMode,

    digital_switches: [bool; 16],
    joystick_axes: [u8; 4],
    in_ack: bool,
}

impl Default for Gamepad {
    fn default() -> Self {
        Self {
            state: Default::default(),
            mode: Default::default(),
            digital_switches: Default::default(),
            joystick_axes: [0x80; 4],
            in_ack: Default::default(),
        }
    }
}

impl Gamepad {
    pub fn send_and_receive_byte(&mut self, data: u8) -> u8 {
        let received = match self.state {
            GamepadState::Init => 0xFF,

            // Gamepad ID: 0x5A73 -> dualshock (analog pad)
            GamepadState::IdLow => self.mode.id()[0],
            GamepadState::IdHigh => self.mode.id()[1],

            // Gamepad switches state
            GamepadState::SwitchLow => self.switch_halfbyte() as u8,
            GamepadState::SwitchHigh => (self.switch_halfbyte() >> 8) as u8,

            GamepadState::AnalogInput0 => self.joystick_axes[StickAxis::RightX as usize],
            GamepadState::AnalogInput1 => self.joystick_axes[StickAxis::RightY as usize],
            GamepadState::AnalogInput2 => self.joystick_axes[StickAxis::LeftX as usize],
            GamepadState::AnalogInput3 => self.joystick_axes[StickAxis::LeftY as usize],
        };

        tracing::debug!(current_state=?self.state, mode=?self.mode, "gamepad got={data:02x} send={received:02x}");

        if let Some(state) = self.mode.next(self.state, data) {
            self.state = state;
            self.in_ack = !matches!(self.state, GamepadState::Init);
            received
        } else {
            // invalid comm sequence
            self.reset();
            0xFF
        }
    }

    pub fn reset(&mut self) {
        tracing::debug!("gamepad reset");
        self.state = GamepadState::Init;
        self.in_ack = false;
    }

    pub fn set_button_state(&mut self, button: Button, pressed: bool) {
        // these buttons only work in analog mode
        match button {
            Button::R3 | Button::L3 if matches!(self.mode, GamepadMode::Digital) => return,
            _ => (),
        }

        self.digital_switches[button as usize] = pressed;
    }

    pub fn set_stick_axis(&mut self, axis: StickAxis, new_value: u8) {
        self.joystick_axes[axis as usize] = new_value;
    }

    pub fn in_ack(&self) -> bool {
        self.in_ack
    }

    pub fn toggle_analog_mode(&mut self) {
        self.mode.toggle();
        self.reset();
    }

    fn switch_halfbyte(&self) -> u16 {
        let mut v = 0u16;
        for i in 0..16 {
            v |= (!self.digital_switches[i] as u16) << i;
        }
        v
    }
}

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

#[derive(Debug, Default, Clone, Copy)]
enum GamepadMode {
    #[default]
    Digital,
    Analog,
}

impl GamepadMode {
    // valid next gamepad comm sequences alongside an optional check byte
    const GAMEPAD_DIGITAL_STATES: [(GamepadState, Option<u8>); 5] = [
        (GamepadState::Init, None),
        (GamepadState::IdLow, Some(0x01)),
        (GamepadState::IdHigh, Some(0x42)),
        (GamepadState::SwitchLow, Some(0x00)),
        (GamepadState::SwitchHigh, None),
    ];

    const GAMEPAD_ANALOG_STATES: [(GamepadState, Option<u8>); 9] = [
        (GamepadState::Init, Some(0x00)),
        (GamepadState::IdLow, Some(0x01)),
        (GamepadState::IdHigh, Some(0x42)),
        (GamepadState::SwitchLow, Some(0x00)),
        (GamepadState::SwitchHigh, None),
        (GamepadState::AnalogInput0, None),
        (GamepadState::AnalogInput1, Some(0x00)),
        (GamepadState::AnalogInput2, Some(0x00)),
        (GamepadState::AnalogInput3, Some(0x00)),
    ];

    fn id(&self) -> [u8; 2] {
        match self {
            GamepadMode::Digital => 0x5A41_u16.to_le_bytes(),
            GamepadMode::Analog => 0x5A73_u16.to_le_bytes(),
        }
    }

    fn toggle(&mut self) {
        *self = match self {
            GamepadMode::Digital => GamepadMode::Analog,
            GamepadMode::Analog => GamepadMode::Digital,
        };
    }

    fn states_table(&self) -> &'static [(GamepadState, Option<u8>)] {
        match self {
            GamepadMode::Digital => &GamepadMode::GAMEPAD_DIGITAL_STATES,
            GamepadMode::Analog => &GamepadMode::GAMEPAD_ANALOG_STATES,
        }
    }

    fn next(&self, current_state: GamepadState, received_byte: u8) -> Option<GamepadState> {
        let idx = current_state as usize;
        let states_table = self.states_table();

        let (next_state, check_byte) = states_table[(idx + 1) % states_table.len()];

        // TAP byte should be zero, multiplayer mode not supported
        if matches!(next_state, GamepadState::SwitchLow) {
            assert_eq!(received_byte, check_byte.unwrap())
        }

        let next_state_is_valid = check_byte.is_none_or(|b| b == received_byte);
        next_state_is_valid.then_some(next_state)
    }
}

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

#[repr(C)]
pub enum StickAxis {
    RightX,
    RightY,
    LeftX,
    LeftY,
}
