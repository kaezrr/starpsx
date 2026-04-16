# StarPSX

StarPSX is a fast, cross-platform PlayStation 1 emulator written entirely in Rust with no external dependencies, making it exceptionally easy to build and highly portable. Available on Linux, Windows, and macOS.

I primarily develop on Linux, so other platform support may be rough at times; feel free to open an issue.

## Showcase

<p align="center">
  <img src="/showcase/crash2.png" width="48%" />
  <img src="/showcase/deadoralive.png" width="48%" /><br/>
  <img src="/showcase/metalgearsolid.png" width="48%" />
  <img src="/showcase/finalf7.png" width="48%" /><br/>
  <img src="/showcase/sfalpha3.png" width="48%" />
  <img src="/showcase/revil3.png" width="48%" />
</p>

<p align="center">
  <b>See the <a href="https://github.com/kaezrr/starpsx/wiki/Compatibility">Compatibility Wiki</a> for the current list of running games.</b>
</p>

## Installation

Download the latest binaries from the [official releases](https://github.com/kaezrr/starpsx/releases/latest) or if you are using an Arch Linux based system, its available on the AUR as the `starpsx-bin` package (it may be outdated because I maintain the package in my free time.)

## Building

StarPSX is designed to be lightweight. On Windows and macOS, no additional dependencies are required.

```sh
cargo build --release
```

On Linux, ensure the following development packages are installed:

```sh
sudo apt install libudev-dev libasound2-dev
```

## Running

StarPSX requires a PlayStation BIOS image to run. On first launch, go to
**Settings > BIOS Settings** and select your BIOS file.

The recommended BIOS is **SCPH-1001** (NTSC-U), this is the version all games
are tested against. Other BIOS versions may work but are untested.

StarPSX defaults to a GUI but also supports CLI-based startup:

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
- **`renderer`**: Software rasterizer written from scratch.
- **`frontend`**: The main user interface crate implemented using `eframe` / `egui`.
- **`cue`**: Cue parsing library written from scratch.
- **`procmac`**: Helpful procedural macros.

## Component Status

| Component   | Status | Notes                                                |
| ----------- | :----: | ---------------------------------------------------- |
| CPU         |   🟢   | Passes most test ROMs                                |
| GPU         |   🟡   | Works well with some minor visual bugs in some games |
| DMA         |   🟡   | Issues with IRQ timings and infinite linked lists    |
| Timers      |   🟢   | Functional but may contain inaccuracies              |
| CDROM       |   🟡   | Some unimplemented commands and timing issues        |
| GTE         |   🟢   | Passes all test ROMs except timing                   |
| Gamepad     |   🟢   | Full analog pad support                              |
| Memory Card |   🟢   | Per-title and shared cards implemented               |
| MDEC        |   🟢   | Passes most test ROMs                                |
| SPU         |   🟡   | No sweep volume implementation                       |

## Acknowledgements

- [psx-spx](https://psx-spx.consoledev.net/) for PlayStation documentation
- [jsgroth's PSX emulator blog posts](https://jsgroth.dev/blog/posts/)
- [Duckstation](https://github.com/stenzek/duckstation) for behavior comparison
- [The folks over at the EmuDev Discord](https://discord.gg/muWhAGteq8)
- [Rustation](https://gitlab.com/flio/rustation-ng/) for GTE reference
