use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use gilrs::Axis as GAxis;
use gilrs::Button as GButton;
use serde::{Deserialize, Serialize};
use starpsx_core::gamepad;
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

        let config_path = resolve_config_path();
        let mut app_config = AppConfig::load_from_file(&config_path);

        if args.show_vram {
            app_config.display_vram = true;
        }

        Ok(Self {
            app_config,
            runnable_path,
            auto_run: args.auto_run,
            config_path,
        })
    }
}

fn resolve_config_path() -> PathBuf {
    std::env::current_exe()
        .expect("exe path")
        .parent()
        .unwrap()
        .join("config.toml")
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub bios_path: Option<PathBuf>,
    pub display_vram: bool,

    #[serde(skip)]
    pub keybinds: input::Bindings,
}

impl AppConfig {
    pub fn load_from_file(path: &Path) -> Self {
        if !path.exists() {
            warn!("no config file detected, writing a default one");
            let cfg = AppConfig::default();
            cfg.save_to_file(path);
            return cfg;
        }

        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(err) => {
                warn!(%err, "failed to read config file, using defaults");
                return AppConfig::default();
            }
        };

        toml::from_str(&text).unwrap_or_else(|err| {
            warn!(%err, "config file was invalid, using a default config");
            AppConfig::default()
        })
    }

    pub fn save_to_file(&self, path: &Path) {
        if let Ok(toml_str) = toml::to_string_pretty(self) {
            let _ = std::fs::write(path, toml_str);
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut keybinds = HashMap::new();
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::South),
            Action::GamepadButton(gamepad::Button::Cross),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::East),
            Action::GamepadButton(gamepad::Button::Circle),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::North),
            Action::GamepadButton(gamepad::Button::Triangle),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::West),
            Action::GamepadButton(gamepad::Button::Square),
        );

        // Shoulders
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::LeftTrigger),
            Action::GamepadButton(gamepad::Button::L1),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::LeftTrigger2),
            Action::GamepadButton(gamepad::Button::L2),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::RightTrigger),
            Action::GamepadButton(gamepad::Button::R1),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::RightTrigger2),
            Action::GamepadButton(gamepad::Button::R2),
        );

        // Menu
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::Select),
            Action::GamepadButton(gamepad::Button::Select),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::Start),
            Action::GamepadButton(gamepad::Button::Start),
        );

        // Stick buttons
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::LeftThumb),
            Action::GamepadButton(gamepad::Button::L3),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::RightThumb),
            Action::GamepadButton(gamepad::Button::R3),
        );

        // Dpad
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::DPadUp),
            Action::GamepadButton(gamepad::Button::Up),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::DPadRight),
            Action::GamepadButton(gamepad::Button::Right),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::DPadDown),
            Action::GamepadButton(gamepad::Button::Down),
        );
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::DPadLeft),
            Action::GamepadButton(gamepad::Button::Left),
        );

        // Analog Sticks
        keybinds.insert(
            PhysicalInput::GilrsAxis(GAxis::LeftStickX),
            Action::StickAxis(gamepad::Axis::LeftX),
        );
        keybinds.insert(
            PhysicalInput::GilrsAxis(GAxis::LeftStickY),
            Action::StickAxis(gamepad::Axis::LeftY),
        );
        keybinds.insert(
            PhysicalInput::GilrsAxis(GAxis::RightStickX),
            Action::StickAxis(gamepad::Axis::RightX),
        );
        keybinds.insert(
            PhysicalInput::GilrsAxis(GAxis::RightStickY),
            Action::StickAxis(gamepad::Axis::RightY),
        );

        // Analog mode toggle
        keybinds.insert(
            PhysicalInput::GilrsButton(GButton::Mode),
            Action::AnalogModeButton,
        );

        Self {
            bios_path: None,
            keybinds,
            display_vram: false,
        }
    }
}
