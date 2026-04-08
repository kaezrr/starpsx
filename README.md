# StarPSX

StarPSX is a free, open-source PlayStation 1 emulator written in Rust. It features a native **cross-platform GUI** (`eframe`/`egui`) with a built-in debugger.

<div align="center">
  <img src="/showcase/mk2-logo.png" width="45%" alt="Mortal Kombat 2"/>
  <img src="/showcase/spyro.png" width="45%" alt="Spyro"/>
  <img src="/showcase/crash.png" width="45%" alt="Crash Bandicoot"/>
  <img src="/showcase/ridge-racer.png" width="45%" alt="Ridge Racer"/>
  <br/>
  <br/>
  
  **See the [Compatibility Wiki](https://github.com/kaezrr/starpsx/wiki/Compatibility) for a list of working games**
</div>

## Installation

Download the latest binaries from the [official releases](https://github.com/kaezrr/starpsx/releases/latest) or if you are using an Arch Linux based system, its available on the AUR as the `starpsx-bin` package.

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
  -f, --full-speed     Run emulator at full speed
  -h, --help           Print help
  -V, --version        Print version
```

## Project Structure

<div align="center">
  <img src="/showcase/project-arch.excalidraw.svg" width="65%" alt="Project Architecture"/>
</div>

- **`core`**: Frontend-agnostic library containing the main emulator logic.
- **`renderer`**: Software rasterizer written from scratch (hardware backend planned).
- **`frontend`**: The main user interface crate implemented using `eframe` / `egui`.

## Component Status

| Component   | Status | Notes                                             |
| ----------- | :----: | ------------------------------------------------- |
| CPU         |   🟢   | Passes most test ROMs                             |
| GPU         |   🟢   | Works well with some minor bugs                   |
| DMA         |   🟡   | Issues with IRQ timings and infinite linked lists |
| Timers      |   🟢   | Functional but may contain inaccuracies           |
| CDROM       |   🟡   | Some unimplemented commands and timing issues     |
| GTE         |   🟢   | Passes all tests                                  |
| Gamepad     |   🟢   | Full analog pad support                           |
| Memory Card |   🟢   | Per-title and shared modes available              |
| MDEC        |   🟢   | Pass most test ROMs                               |
| SPU         |   🟡   | No reverb or sweep implementation                 |

## Acknowledgements

- psx-spx for PlayStation documentation
- jsgroth’s PSX emulator blog posts
- duckstation for behavior comparison
- the folks over at the EmuDev Discord
- Lionel Flandrin's Rustation for GTE reference
