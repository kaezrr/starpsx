use starpsx_core::gamepad;

// Logs to a fixed path for now
pub fn initialize_logging() -> Result<(), String> {
    std::fs::create_dir_all("./logs")
        .map_err(|e| format!("failed to create logs directory: {e}"))?;
    std::fs::File::create("./logs/psx.log")
        .map_err(|e| format!("failed to initialize log file: {e}"))?;
    Ok(())
}

pub fn convert_axis(axis: gilrs::Axis, value: f32) -> (gamepad::StickAxis, u8) {
    // Y axis is flipped between gilrs and console
    let v = match axis {
        gilrs::Axis::LeftStickY | gilrs::Axis::RightStickY => -value,
        _ => value,
    };

    let byte = ((v + 1.0) * 127.5).round().clamp(0.0, 255.0) as u8;

    let mapped = match axis {
        gilrs::Axis::RightStickX => gamepad::StickAxis::RightX,
        gilrs::Axis::RightStickY => gamepad::StickAxis::RightY,

        gilrs::Axis::LeftStickX => gamepad::StickAxis::LeftX,
        gilrs::Axis::LeftStickY => gamepad::StickAxis::LeftY,

        _ => unimplemented!("unmapped gamepad axis"),
    };

    (mapped, byte)
}

pub fn convert_button(gilrs_button: gilrs::Button) -> gamepad::Button {
    match gilrs_button {
        // Face buttons
        gilrs::Button::South => gamepad::Button::Cross,
        gilrs::Button::East => gamepad::Button::Circle,
        gilrs::Button::North => gamepad::Button::Triangle,
        gilrs::Button::West => gamepad::Button::Square,

        // Shoulders / Triggers
        gilrs::Button::LeftTrigger => gamepad::Button::L1,
        gilrs::Button::LeftTrigger2 => gamepad::Button::L2,
        gilrs::Button::RightTrigger => gamepad::Button::R1,
        gilrs::Button::RightTrigger2 => gamepad::Button::R2,

        // Menu
        gilrs::Button::Select => gamepad::Button::Select,
        gilrs::Button::Start => gamepad::Button::Start,

        // Thumbsticks
        gilrs::Button::LeftThumb => gamepad::Button::L3,
        gilrs::Button::RightThumb => gamepad::Button::R3,

        // D-Pad
        gilrs::Button::DPadUp => gamepad::Button::Up,
        gilrs::Button::DPadDown => gamepad::Button::Down,
        gilrs::Button::DPadLeft => gamepad::Button::Left,
        gilrs::Button::DPadRight => gamepad::Button::Right,

        _ => unimplemented!("unmapped gamepad button"),
    }
}
