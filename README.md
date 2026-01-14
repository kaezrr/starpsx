# StarPSX

StarPSX is a free and open-source PlayStation 1 emulator written in Rust.
This project is **highly work in progress** and not yet suitable for playing games end-to-end.

StarPSX now features a native **cross-platform GUI frontend** built using `eframe` / `egui`, with a CLI mode available for automation and development workflows.

---

## Screenshots

<div align="center" class="grid" markdown>
  <img src="/showcase/mk2-logo.png" width="45%"/>
  <img src="/showcase/ewj2-logo.png" width="45%"/>
</div>

---

## Installation & Usage

### Installation

**GitHub:**
Download the latest binaries from the [official releases](https://github.com/kaezrr/starpsx/releases/latest).

**Arch Linux (AUR):**

```sh
paru -S starpsx-bin
```

---

### Building from Source

StarPSX aims to remain self-contained and depends only on the Rust toolchain.

```sh
cargo build --release
```

---

### Execution

#### GUI Mode (default)

Running the binary without arguments launches the GUI frontend:

```sh
starpsx
```

From the GUI you can:

- Select a BIOS image
- Load a game or PS-EXE
- Control emulator state (pause / restart)
- View runtime metrics

---

#### CLI / Auto-Run Mode

The CLI can be used to skip the GUI and immediately start emulation:

```sh
starpsx --auto-run
starpsx --auto-run path/to/game.bin
```

- With `--auto-run` only: boots using the configured BIOS
- With `--auto-run <file>`: boots directly into the given game or PS-EXE

This mode is intended for:

- Development
- Automation
- Testing

---

## Project Structure

The project is split into three main crates:

- **core**
  A library crate that hosts the main emulator logic (CPU, GPU, DMA, timers, etc).
  This crate is frontend-agnostic and can be reused by different UIs.

- **renderer**
  A software rasterizer written from scratch.
  Used by `core` to draw into a framebuffer.
  Support for multiple rendering backends (software / hardware) is planned.

- **frontend**
  The main binary crate providing the user interface.
  Currently implemented using **`eframe` / `egui`**, handling.

---

## Component Status

| Component   | Status      | Notes                                   |
| ----------- | ----------- | --------------------------------------- |
| CPU         | Done        | passes most test ROMs                   |
| GPU         | Done        | works well with some bugs               |
| DMA         | Partial     | burst and linked-list DMA supported     |
| Timers      | Done        | functional but may contain inaccuracies |
| CDROM       | Partial     | boots a few games                       |
| Gamepad     | Done        | full analog pad support                 |
| Memory Card | Not started |                                         |
| SPU         | Not started |                                         |
| GTE         | Not started |                                         |
| MDEC        | Not started |                                         |

---

## A Lot Of Thanks To

- psx-spx for PlayStation documentation
- jsgroth’s PSX emulator blog posts
- duckstation for behavior comparison
- The folks over at the EmuDev Discord
