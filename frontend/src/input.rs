use std::collections::HashMap;

use eframe::egui::Key as EKey;
use gilrs::Axis as GAxis;
use gilrs::Button as GButton;
use starpsx_core::gamepad::Axis;
use starpsx_core::gamepad::Button;
use tracing::error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GamepadState {
    pub buttons: u16,
    pub left_stick: (u8, u8),
    pub right_stick: (u8, u8),
    pub analog_mode: bool,
}

impl Default for GamepadState {
    fn default() -> Self {
        Self {
            buttons: 0xFFFF,
            left_stick: (0x80, 0x80),
            right_stick: (0x80, 0x80),
            analog_mode: false,
        }
    }
}

impl GamepadState {
    pub fn handle_action(&mut self, action: Action, value: ActionValue) -> bool {
        let before = self.clone();

        match (action, value) {
            (Action::StickAxis(axis), ActionValue::Analog(v)) => {
                self.update_axis(axis, v);
            }

            (Action::GamepadButton(button), ActionValue::Digital(pressed)) => {
                self.update_button(button, pressed);
            }
            // Might have issues with latching
            (Action::AnalogModeButton, ActionValue::Digital(pressed)) => {
                if !pressed {
                    self.analog_mode = !self.analog_mode;
                }
            }

            (Action::DigitalAxisPositive(axis), ActionValue::Digital(true)) => {
                self.update_axis(axis, 1.0);
            }

            (Action::DigitalAxisNegative(axis), ActionValue::Digital(true)) => {
                self.update_axis(axis, -1.0);
            }

            (
                Action::DigitalAxisPositive(axis) | Action::DigitalAxisNegative(axis),
                ActionValue::Digital(false),
            ) => {
                self.update_axis(axis, 0.0);
            }

            (_, _) => error!("invalid action and value pair"),
        }

        *self != before
    }

    fn update_axis(&mut self, axis: Axis, value: f32) {
        // Left stick maps to dpad in digital mode
        if !self.analog_mode {
            const DIGITAL_THRESHOLD: f32 = 0.6;
            match axis {
                Axis::LeftX => {
                    self.update_button(Button::Left, value < -DIGITAL_THRESHOLD);
                    self.update_button(Button::Right, value > DIGITAL_THRESHOLD);
                }

                Axis::LeftY => {
                    self.update_button(Button::Up, value > DIGITAL_THRESHOLD);
                    self.update_button(Button::Down, value < -DIGITAL_THRESHOLD);
                }

                _ => {}
            }

            return;
        }

        // Y axis is flipped between gilrs and console
        let v = match axis {
            Axis::LeftY | Axis::RightY => -value,
            _ => value,
        };

        let byte = v.mul_add(127.5, 127.5) as u8;

        match axis {
            Axis::RightX => self.right_stick.0 = byte,
            Axis::RightY => self.right_stick.1 = byte,

            Axis::LeftX => self.left_stick.0 = byte,
            Axis::LeftY => self.left_stick.1 = byte,
        }
    }

    fn update_button(&mut self, button: Button, pressed: bool) {
        let mask = 1 << (button as usize);
        debug_assert!(mask <= 0x8000);

        if pressed {
            self.buttons &= !mask;
        } else {
            self.buttons |= mask;
        }
    }
}

pub type Bindings = HashMap<PhysicalInput, Action>;

#[derive(Clone, Copy)]
pub enum Action {
    StickAxis(Axis),
    GamepadButton(Button),
    AnalogModeButton,
    DigitalAxisPositive(Axis),
    DigitalAxisNegative(Axis),
}

#[derive(Clone, Copy)]
pub enum ActionValue {
    Digital(bool),
    Analog(f32),
}

#[derive(Eq, PartialEq, Hash)]
pub enum PhysicalInput {
    GilrsButton(GButton),
    GilrsAxis(GAxis),
    Key(EKey),
}
