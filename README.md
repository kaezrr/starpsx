# StarPSX

StarPSX is a free and open-source PlayStation 1 emulator written in Rust.  
This project is highly work in progress and not suitable for playing games yet.

## Installation

You can download the [latest release](https://github.com/kaezrr/starpsx/releases/latest) from Github.  
StarPSX is also available on the AUR, for Arch-based distros:

```sh
paru -S starpsx-bin
```

## Project Structure

The project is split into three crates:

- **core**  
  A library crate that hosts the main emulator functionality.

- **renderer**  
  A software rasterizer written from scratch.  
  This is a library crate used by `core` for drawing to a framebuffer.  
  In the future, support for switchable renderers is planned (e.g. software vs hardware backends).

- **frontend**  
  The main binary crate that provides the frontend GUI.  
  Currently, it uses a basic `winit` + `softbuffer` window for output.  
  The long-term plan is to integrate a fully featured GUI framework.

## Component Status

| Component | Status      | Notes                                |
| --------- | ----------- | ------------------------------------ |
| CPU       | Done        | Passes most test roms                |
| GPU       | Done        | Might have some bugs                 |
| DMA       | Partial     | Only GPU port implemented            |
| Timers    | Done        | Roughly works but it might have bugs |
| CDROM     | Partial     | Just enough to get to the shell      |
| Gamepad   | Partial     | Start working on the serial ports    |
| SPU       | Not started |                                      |
| GTE       | Not started |                                      |
| MDEC      | Not started |                                      |

## Current Status

StarPSX is in the very early stages of development.  
At the moment, it can boot the BIOS and run some basic test roms.

## Build Instructions

```sh
cargo build --release

```

## Running Instructions

```sh
starpsx <path/to/bios> <path/to/exe>(optional)

```

- `<path/to/bios>` should point to a valid PlayStation BIOS image.
- `<path/to/exe>` is optional and can be used to sideload an EXE file.

## Dependencies

StarPSX depends only on the Rust toolchain.
The project aims to avoid external dependencies and remain self-contained.

## Special Thanks

- psx-spx for their wonderful PlayStation documentation
- jsgroth's PSX blogs for detailed write-ups on PSX emulators.
- DuckStation for comparing correct behaviors
- The folks over at the EmuDev Discord for always being helpful
- PeterLemon and Amidog for their invaluable test ROMs
