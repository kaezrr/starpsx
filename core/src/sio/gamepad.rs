pub struct Gamepad {
    state: State,
    mode: Mode,

    digital_switches: u16,
    joystick_axes: [u8; 4],
    in_ack: bool,
}

impl Default for Gamepad {
    fn default() -> Self {
        Self {
            state: Default::default(),
            mode: Default::default(),
            digital_switches: 0xFFFF,
            joystick_axes: [0x80; 4],
            in_ack: Default::default(),
        }
    }
}

impl Gamepad {
    pub fn send_and_receive_byte(&mut self, data: u8) -> u8 {
        let received = match self.state {
            State::Init => 0xFF,

            // Gamepad ID: 0x5A73 -> dualshock (analog pad)
            State::IdLow => self.mode.id()[0],
            State::IdHigh => self.mode.id()[1],

            // Gamepad switches state
            State::SwitchLow => self.digital_switches as u8,
            State::SwitchHigh => (self.digital_switches >> 8) as u8,

            State::AnalogInput0 => self.joystick_axes[Axis::RightX as usize],
            State::AnalogInput1 => self.joystick_axes[Axis::RightY as usize],
            State::AnalogInput2 => self.joystick_axes[Axis::LeftX as usize],
            State::AnalogInput3 => self.joystick_axes[Axis::LeftY as usize],
        };

        if let Some(state) = self.mode.next(self.state, data) {
            self.state = state;
            self.in_ack = self.state != State::Init;
            received
        } else {
            // invalid comm sequence
            self.reset();
            0xFF
        }
    }

    pub fn reset(&mut self) {
        self.state = State::Init;
        self.in_ack = false;
    }

    pub fn set_stick_axis(&mut self, left: (u8, u8), right: (u8, u8)) {
        self.joystick_axes = [right.0, right.1, left.0, left.1];
    }

    pub fn set_buttons(&mut self, new_value: u16) {
        self.digital_switches = new_value;
    }

    pub fn set_analog_mode(&mut self, in_analog: bool) {
        if self.mode.get() == in_analog {
            return;
        }
        self.mode.set(in_analog);
        self.reset();
    }

    pub fn in_ack(&self) -> bool {
        self.in_ack
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum State {
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
enum Mode {
    #[default]
    Digital,
    Analog,
}

impl Mode {
    // valid next gamepad comm sequences alongside an optional check byte
    const GAMEPAD_DIGITAL_STATES: [(State, Option<u8>); 5] = [
        (State::Init, None),
        (State::IdLow, Some(0x01)),
        (State::IdHigh, Some(0x42)),
        (State::SwitchLow, Some(0x00)),
        (State::SwitchHigh, None),
    ];

    const GAMEPAD_ANALOG_STATES: [(State, Option<u8>); 9] = [
        (State::Init, Some(0x00)),
        (State::IdLow, Some(0x01)),
        (State::IdHigh, Some(0x42)),
        (State::SwitchLow, Some(0x00)),
        (State::SwitchHigh, None),
        (State::AnalogInput0, None),
        (State::AnalogInput1, Some(0x00)),
        (State::AnalogInput2, Some(0x00)),
        (State::AnalogInput3, Some(0x00)),
    ];

    fn id(&self) -> [u8; 2] {
        match self {
            Mode::Digital => 0x5A41_u16.to_le_bytes(),
            Mode::Analog => 0x5A73_u16.to_le_bytes(),
        }
    }

    fn set(&mut self, in_analog: bool) {
        *self = match in_analog {
            false => Mode::Digital,
            true => Mode::Analog,
        };
    }

    fn get(&self) -> bool {
        match self {
            Mode::Digital => false,
            Mode::Analog => true,
        }
    }

    fn states_table(&self) -> &'static [(State, Option<u8>)] {
        match self {
            Mode::Digital => &Mode::GAMEPAD_DIGITAL_STATES,
            Mode::Analog => &Mode::GAMEPAD_ANALOG_STATES,
        }
    }

    fn next(&self, current_state: State, received_byte: u8) -> Option<State> {
        let idx = current_state as usize;
        let states_table = self.states_table();

        let (next_state, check_byte) = states_table[(idx + 1) % states_table.len()];

        // TAP byte should be zero, multiplayer mode not supported
        if matches!(next_state, State::SwitchLow) {
            assert_eq!(received_byte, check_byte.unwrap())
        }

        let next_state_is_valid = check_byte.is_none_or(|b| b == received_byte);
        next_state_is_valid.then_some(next_state)
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
pub enum Axis {
    RightX,
    RightY,
    LeftX,
    LeftY,
}
