# StarPSX

StarPSX is a free, open-source PlayStation 1 emulator written in Rust. It features a native **cross-platform GUI** (`eframe`/`egui`) with a built-in debugger.

> [!WARNING]
>
> This project is currently **highly work in progress** and is not yet suitable for playing games end-to-end. Many features are incomplete or missing.

<div align="center">
  <img src="/showcase/mk2-logo.png" width="45%" alt="Mortal Kombat 2"/>
  <img src="/showcase/ewj2-logo.png" width="45%" alt="Earthworm Jim 2"/>
  <br/>
  <br/>
  
  📖 **See the [Compatibility Wiki](https://github.com/kaezrr/starpsx/wiki/Compatibility) for a list of working games**
</div>

## Installation

Download the latest binaries from the [official releases](https://github.com/kaezrr/starpsx/releases/latest) or if you are using Arch Linux based system, its available on the AUR as the `starpsx-bin` package.

## Building

StarPSX is designed to be lightweight. On Windows and macOS, no additional dependencies are required.

```sh
cargo build --release
```

On Linux, ensure the following development packages are installed:

```sh
sudo apt install libudev-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libasound2-dev
```

## Usage

StarPSX defaults to a GUI but also supports CLI-based usage.

```
Usage: starpsx [OPTIONS] [FILE]

Arguments:
  [FILE]  File to start the emulator with

Options:
  -s, --show-vram      Display full VRAM
  -a, --auto-run       Skip GUI and auto-start the emulator
  -d, --debugger-view  Show debugger_view on startup
  -h, --help           Print help
  -V, --version        Print version
```

> [!NOTE]
> If you have multiple GPUs, you can use the environment variable `WGPU_POWER_PREF` (values: `none`, `low`, `high`) to force the application to use a specific device.

## Project Structure

- **`core`**: Frontend-agnostic library containing the main emulator logic.
- **`renderer`**: Software rasterizer written from scratch (hardware backend planned).
- **`frontend`**: The main user interface crate implemented using `eframe` / `egui`.

## Acknowledgements

- psx-spx for PlayStation documentation
- jsgroth’s PSX emulator blog posts
- duckstation for behavior comparison
- The folks over at the EmuDev Discord
