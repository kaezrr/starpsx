use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use eframe::egui::Key as EKey;
use gilrs::Axis as GAxis;
use gilrs::Button as GButton;
use serde::{Deserialize, Serialize};
use starpsx_core::gamepad;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::input;
use crate::input::Action;
use crate::input::PhysicalInput;

pub enum RunnablePath {
    Exe(PathBuf),
    Bin(PathBuf),
}

/// Cross Platform PS1 Emulator written in Rust
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Display full VRAM
    #[arg(short, long)]
    show_vram: bool,

    /// Skip GUI and auto-start the emulator
    #[arg(short, long)]
    auto_run: bool,

    /// Show debugger_view on startup
    #[arg(short, long)]
    debugger_view: bool,

    /// File to start the emulator with
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
}

pub struct LaunchConfig {
    pub app_config: AppConfig,
    pub runnable_path: Option<PathBuf>,
    pub auto_run: bool,
    pub config_path: PathBuf,
}

impl LaunchConfig {
    pub fn build() -> Result<Self, Box<dyn Error>> {
        let args = Args::parse();
        let runnable_path = args.file;

        let config_path = dirs::config_dir()
            .ok_or("could not find config directory")?
            .join("StarPSX")
            .join("config.toml");

        let mut app_config = AppConfig::load_from_file(&config_path)
            .with_default_controller()
            .with_default_keyboard();

        if args.show_vram {
            app_config.display_vram = true;
        }

        if args.debugger_view {
            app_config.debugger_view = true;
        }

        Ok(Self {
            app_config,
            runnable_path,
            auto_run: args.auto_run,
            config_path,
        })
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub bios_path: Option<PathBuf>,
    pub display_vram: bool,
    pub debugger_view: bool,

    #[serde(skip)]
    pub keybinds: input::Bindings,
}

impl AppConfig {
    pub fn load_from_file(path: &Path) -> Self {
        if !path.exists() {
            warn!("no config file detected, writing a default one");

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create config dir");
            }

            let cfg = AppConfig::default();
            cfg.save_to_file(path);

            return cfg;
        }

        let text = match std::fs::read_to_string(path) {
            Ok(t) => {
                info!(?path, "found a valid config file at");
                t
            }
            Err(err) => {
                error!(%err, "failed to read config file, using defaults");
                return AppConfig::default();
            }
        };

        toml::from_str(&text).unwrap_or_else(|err| {
            error!(%err, "config file was invalid, using a default config");
            AppConfig::default()
        })
    }

    pub fn save_to_file(&self, path: &Path) {
        if let Ok(toml_str) = toml::to_string_pretty(self) {
            info!(?path, "saving config file to");
            let _ = std::fs::write(path, toml_str);
        }
    }

    fn with_default_controller(mut self) -> Self {
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::South),
            Action::GamepadButton(gamepad::Button::Cross),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::East),
            Action::GamepadButton(gamepad::Button::Circle),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::North),
            Action::GamepadButton(gamepad::Button::Triangle),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::West),
            Action::GamepadButton(gamepad::Button::Square),
        );

        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::LeftTrigger),
            Action::GamepadButton(gamepad::Button::L1),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::LeftTrigger2),
            Action::GamepadButton(gamepad::Button::L2),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::RightTrigger),
            Action::GamepadButton(gamepad::Button::R1),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::RightTrigger2),
            Action::GamepadButton(gamepad::Button::R2),
        );

        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::Select),
            Action::GamepadButton(gamepad::Button::Select),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::Start),
            Action::GamepadButton(gamepad::Button::Start),
        );

        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::LeftThumb),
            Action::GamepadButton(gamepad::Button::L3),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::RightThumb),
            Action::GamepadButton(gamepad::Button::R3),
        );

        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::DPadUp),
            Action::GamepadButton(gamepad::Button::Up),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::DPadRight),
            Action::GamepadButton(gamepad::Button::Right),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::DPadDown),
            Action::GamepadButton(gamepad::Button::Down),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::DPadLeft),
            Action::GamepadButton(gamepad::Button::Left),
        );

        self.keybinds.insert(
            PhysicalInput::GilrsAxis(GAxis::LeftStickX),
            Action::StickAxis(gamepad::Axis::LeftX),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsAxis(GAxis::LeftStickY),
            Action::StickAxis(gamepad::Axis::LeftY),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsAxis(GAxis::RightStickX),
            Action::StickAxis(gamepad::Axis::RightX),
        );
        self.keybinds.insert(
            PhysicalInput::GilrsAxis(GAxis::RightStickY),
            Action::StickAxis(gamepad::Axis::RightY),
        );

        self.keybinds.insert(
            PhysicalInput::GilrsButton(GButton::Mode),
            Action::AnalogModeButton,
        );

        self
    }

    fn with_default_keyboard(mut self) -> Self {
        self.keybinds.insert(
            PhysicalInput::Key(EKey::K),
            Action::GamepadButton(gamepad::Button::Cross),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::L),
            Action::GamepadButton(gamepad::Button::Circle),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::I),
            Action::GamepadButton(gamepad::Button::Triangle),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::J),
            Action::GamepadButton(gamepad::Button::Square),
        );

        self.keybinds.insert(
            PhysicalInput::Key(EKey::Q),
            Action::GamepadButton(gamepad::Button::L1),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::Num1),
            Action::GamepadButton(gamepad::Button::L2),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::E),
            Action::GamepadButton(gamepad::Button::R1),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::Num3),
            Action::GamepadButton(gamepad::Button::R2),
        );

        self.keybinds.insert(
            PhysicalInput::Key(EKey::Backspace),
            Action::GamepadButton(gamepad::Button::Select),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::Enter),
            Action::GamepadButton(gamepad::Button::Start),
        );

        self.keybinds.insert(
            PhysicalInput::Key(EKey::Num2),
            Action::GamepadButton(gamepad::Button::L3),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::Num4),
            Action::GamepadButton(gamepad::Button::R3),
        );

        self.keybinds.insert(
            PhysicalInput::Key(EKey::ArrowUp),
            Action::GamepadButton(gamepad::Button::Up),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::ArrowRight),
            Action::GamepadButton(gamepad::Button::Right),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::ArrowDown),
            Action::GamepadButton(gamepad::Button::Down),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::ArrowLeft),
            Action::GamepadButton(gamepad::Button::Left),
        );

        self.keybinds.insert(
            PhysicalInput::Key(EKey::A),
            Action::DigitalAxisNegative(gamepad::Axis::LeftX),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::D),
            Action::DigitalAxisPositive(gamepad::Axis::LeftX),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::W),
            Action::DigitalAxisPositive(gamepad::Axis::LeftY),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::S),
            Action::DigitalAxisNegative(gamepad::Axis::LeftY),
        );

        self.keybinds.insert(
            PhysicalInput::Key(EKey::F),
            Action::DigitalAxisNegative(gamepad::Axis::RightX),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::H),
            Action::DigitalAxisPositive(gamepad::Axis::RightX),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::T),
            Action::DigitalAxisPositive(gamepad::Axis::RightY),
        );
        self.keybinds.insert(
            PhysicalInput::Key(EKey::G),
            Action::DigitalAxisNegative(gamepad::Axis::RightY),
        );

        self.keybinds
            .insert(PhysicalInput::Key(EKey::M), Action::AnalogModeButton);

        self
    }
}

pub struct KeybindRow {
    pub action: &'static str,
    pub keyboard: &'static str,
    pub controller: &'static str,
}

pub const KEYBIND_ROWS: &[KeybindRow] = &[
    // Face buttons
    KeybindRow {
        action: "Cross",
        keyboard: "K",
        controller: "South (A)",
    },
    KeybindRow {
        action: "Circle",
        keyboard: "L",
        controller: "East (B)",
    },
    KeybindRow {
        action: "Triangle",
        keyboard: "I",
        controller: "North (Y)",
    },
    KeybindRow {
        action: "Square",
        keyboard: "J",
        controller: "West (X)",
    },
    // Shoulder buttons
    KeybindRow {
        action: "L1",
        keyboard: "Q",
        controller: "L1",
    },
    KeybindRow {
        action: "L2",
        keyboard: "1",
        controller: "L2",
    },
    KeybindRow {
        action: "R1",
        keyboard: "E",
        controller: "R1",
    },
    KeybindRow {
        action: "R2",
        keyboard: "3",
        controller: "R2",
    },
    // System buttons
    KeybindRow {
        action: "Select",
        keyboard: "Backspace",
        controller: "Select",
    },
    KeybindRow {
        action: "Start",
        keyboard: "Enter",
        controller: "Start",
    },
    // Stick buttons
    KeybindRow {
        action: "L3",
        keyboard: "2",
        controller: "Left Stick Press",
    },
    KeybindRow {
        action: "R3",
        keyboard: "4",
        controller: "Right Stick Press",
    },
    // D-Pad
    KeybindRow {
        action: "D-Pad Up",
        keyboard: "Arrow Up",
        controller: "D-Pad Up",
    },
    KeybindRow {
        action: "D-Pad Right",
        keyboard: "Arrow Right",
        controller: "D-Pad Right",
    },
    KeybindRow {
        action: "D-Pad Down",
        keyboard: "Arrow Down",
        controller: "D-Pad Down",
    },
    KeybindRow {
        action: "D-Pad Left",
        keyboard: "Arrow Left",
        controller: "D-Pad Left",
    },
    // Left stick (digital / analog)
    KeybindRow {
        action: "Left Stick X−",
        keyboard: "A",
        controller: "Left Stick X",
    },
    KeybindRow {
        action: "Left Stick X+",
        keyboard: "D",
        controller: "Left Stick X",
    },
    KeybindRow {
        action: "Left Stick Y+",
        keyboard: "W",
        controller: "Left Stick Y",
    },
    KeybindRow {
        action: "Left Stick Y−",
        keyboard: "S",
        controller: "Left Stick Y",
    },
    // Right stick (digital / analog)
    KeybindRow {
        action: "Right Stick X−",
        keyboard: "F",
        controller: "Right Stick X",
    },
    KeybindRow {
        action: "Right Stick X+",
        keyboard: "H",
        controller: "Right Stick X",
    },
    KeybindRow {
        action: "Right Stick Y+",
        keyboard: "T",
        controller: "Right Stick Y",
    },
    KeybindRow {
        action: "Right Stick Y−",
        keyboard: "G",
        controller: "Right Stick Y",
    },
    // Mode
    KeybindRow {
        action: "Analog Mode",
        keyboard: "M",
        controller: "Mode",
    },
];
