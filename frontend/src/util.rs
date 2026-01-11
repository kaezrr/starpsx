use std::collections::HashMap;

use gilrs::Axis as GAxis;
use gilrs::Button as GButton;
use starpsx_core::gamepad::{self, Axis, Button};
use tracing::error;

#[derive(Clone, Copy)]
pub struct GamepadState {
    pub buttons: u16,
    pub left_stick: (u8, u8),
    pub right_stick: (u8, u8),
    pub analog_mode: bool,
}

impl Default for GamepadState {
    fn default() -> Self {
        Self {
            buttons: 0xFF,
            left_stick: (0x80, 0x80),
            right_stick: (0x80, 0x80),
            analog_mode: false,
        }
    }
}

impl GamepadState {
    pub fn handle_action(&mut self, action: &Action, value: ActionValue) {
        match (action, value) {
            (Action::StickAxis(axis), ActionValue::Analog(v)) => {
                self.update_axis(axis, v);
            }

            (Action::GamepadButton(button), ActionValue::Digital(pressed)) => {
                self.update_button(button, pressed);
            }
            // Might have issues with latching
            (Action::AnalogModeButton, ActionValue::Digital(false)) => {
                self.analog_mode = !self.analog_mode;
            }

            (Action::DigitalAxisPositive(axis), ActionValue::Digital(true)) => {
                self.update_axis(axis, 1.0);
            }

            (Action::DigitalAxisPositive(axis), ActionValue::Digital(false)) => {
                self.update_axis(axis, 0.0);
            }

            (Action::DigitalAxisNegative(axis), ActionValue::Digital(true)) => {
                self.update_axis(axis, -1.0);
            }

            (Action::DigitalAxisNegative(axis), ActionValue::Digital(false)) => {
                self.update_axis(axis, 0.0);
            }

            (_, _) => error!("invalid action and value pair"),
        };
    }

    fn update_axis(&mut self, axis: &Axis, value: f32) {
        // Y axis is flipped between gilrs and console
        let v = match axis {
            Axis::LeftY | Axis::RightY => -value,
            _ => value,
        };

        let byte = ((v + 1.0) * 127.5).round().clamp(0.0, 255.0) as u8;
        match axis {
            Axis::RightX => self.right_stick.0 = byte,
            Axis::RightY => self.right_stick.1 = byte,

            Axis::LeftX => self.left_stick.0 = byte,
            Axis::LeftY => self.left_stick.1 = byte,
        };
    }

    fn update_button(&mut self, button: &Button, pressed: bool) {
        let mask = 1 << (*button as usize);
        debug_assert!(mask <= 0x8000);

        match pressed {
            true => self.buttons &= !mask,
            false => self.buttons |= mask,
        };
    }
}

pub type Bindings = HashMap<PhysicalInput, Action>;

#[allow(unused)]
pub enum Action {
    StickAxis(Axis),
    GamepadButton(Button),
    AnalogModeButton,
    DigitalAxisPositive(Axis),
    DigitalAxisNegative(Axis),
}

pub enum ActionValue {
    Digital(bool),
    Analog(f32),
}

#[allow(unused)]
#[derive(Eq, PartialEq, Hash)]
pub enum PhysicalInput {
    GilrsButton(GButton),
    GilrsAxis(GAxis),
    Key(eframe::egui::Key),
}

pub fn get_default_keybinds() -> Bindings {
    let mut keybindings = HashMap::new();
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::South),
        Action::GamepadButton(gamepad::Button::Cross),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::East),
        Action::GamepadButton(gamepad::Button::Circle),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::North),
        Action::GamepadButton(gamepad::Button::Triangle),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::West),
        Action::GamepadButton(gamepad::Button::Square),
    );

    // Shoulders
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::LeftTrigger),
        Action::GamepadButton(gamepad::Button::L1),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::LeftTrigger2),
        Action::GamepadButton(gamepad::Button::L2),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::RightTrigger),
        Action::GamepadButton(gamepad::Button::R1),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::RightTrigger2),
        Action::GamepadButton(gamepad::Button::R2),
    );

    // Menu
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::Select),
        Action::GamepadButton(gamepad::Button::Select),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::Start),
        Action::GamepadButton(gamepad::Button::Start),
    );

    // Stick buttons
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::LeftThumb),
        Action::GamepadButton(gamepad::Button::L3),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::RightThumb),
        Action::GamepadButton(gamepad::Button::R3),
    );

    // Dpad
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::DPadUp),
        Action::GamepadButton(gamepad::Button::Up),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::DPadRight),
        Action::GamepadButton(gamepad::Button::Right),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::DPadDown),
        Action::GamepadButton(gamepad::Button::Down),
    );
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::DPadLeft),
        Action::GamepadButton(gamepad::Button::Left),
    );

    // Analog Sticks
    keybindings.insert(
        PhysicalInput::GilrsAxis(GAxis::LeftStickX),
        Action::StickAxis(gamepad::Axis::LeftX),
    );
    keybindings.insert(
        PhysicalInput::GilrsAxis(GAxis::LeftStickY),
        Action::StickAxis(gamepad::Axis::LeftY),
    );
    keybindings.insert(
        PhysicalInput::GilrsAxis(GAxis::RightStickX),
        Action::StickAxis(gamepad::Axis::RightX),
    );
    keybindings.insert(
        PhysicalInput::GilrsAxis(GAxis::RightStickY),
        Action::StickAxis(gamepad::Axis::RightY),
    );

    // Analog mode toggle
    keybindings.insert(
        PhysicalInput::GilrsButton(GButton::Mode),
        Action::AnalogModeButton,
    );

    keybindings
}
