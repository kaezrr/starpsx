# StarPSX

StarPSX is a free and open-source PlayStation 1 emulator written in Rust.  
This project is highly work in progress and not suitable for playing games yet.

## Screenshots

<div align="center" class="grid" markdown>
  <img src="/showcase/mk2-logo.png" width="45%"/>
  <img src="/showcase/mk2-fight.png" width="45%"/>
</div>

## Installation & Usage

### Installation

**GitHub:** Download the latest binaries from the [official releases](https://github.com/kaezrr/starpsx/releases/latest).

**Arch Linux (AUR):**

```sh
paru -S starpsx-bin

```

### Building from Source

StarPSX aims to remain self-contained and depends only on the Rust toolchain.

```sh
cargo build --release

```

### Execution

The emulator is currently CLI-based (with plans for a GUI later). Use the following command structure:

```sh
starpsx [path/to/bios] [path/to/runnable](optional)

```

- **BIOS**: Path to a valid PlayStation BIOS image.
- **Runnable**: Optional path to a game (.bin) or a sideloaded .EXE file.

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

| Component   | Status      | Notes                                   |
| ----------- | ----------- | --------------------------------------- |
| CPU         | Done        | passes most test roms                   |
| GPU         | Done        | works well with some bugs               |
| DMA         | Partial     | only burst and linkedlist dma supported |
| Timers      | Done        | roughly works but it might have bugs    |
| CDROM       | Partial     | boots a few games                       |
| Gamepad     | Done        | full analog pad support                 |
| Memory card | Not started |                                         |
| SPU         | Not started |                                         |
| GTE         | Not started |                                         |
| MDEC        | Not started |                                         |

## A Lot Of Thanks To

- psx-spx for their playstation documentation
- jsgroth's psx blogs for detailed write-ups on psx emulators.
- duckstation for comparing correct behaviors
- The folks over at the emudev discord
