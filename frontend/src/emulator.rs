use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Context;
use cpal::StreamConfig;
use cpal::default_host;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use ringbuf::HeapCons;
use ringbuf::HeapProd;
use ringbuf::traits::Consumer;
use ringbuf::traits::Producer;
use ringbuf::traits::Split;
use starpsx_core::SystemSnapshot;
use starpsx_renderer::FrameBuffer;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::config::MediaPath;
use crate::input::GamepadState;

const AUDIO_STREAM_CONFIG: StreamConfig = StreamConfig {
    channels: 2,
    sample_rate: 44100_u32,
    buffer_size: cpal::BufferSize::Default,
};

pub enum UiCommand {
    SetVramDisplay(bool),
    SetSpeed(bool),
    Restart,
    Shutdown,

    DebugSetBreakpoint(u32, bool),
    DebugStep,
    DebugRequestState,
}

pub struct UiChannels {
    pub frame_tx: Sender<FrameBuffer>,
    pub ui_command_rx: Receiver<UiCommand>,
    pub input_rx: Receiver<GamepadState>,
    pub snapshot_tx: Sender<SystemSnapshot>,
}

pub struct Emulator {
    channels: UiChannels,
    shared_state: Arc<SharedState>,
    system: starpsx_core::System,
    breakpoints: HashSet<u32>,
    bios_path: PathBuf,
    file_path: Option<MediaPath>,
    memory_card: Option<PathBuf>,
    show_vram: bool,
    full_speed: bool,
}

impl Emulator {
    pub fn build(
        channels: UiChannels,
        shared_state: Arc<SharedState>,
        bios_path: PathBuf,
        file_path: Option<MediaPath>,
        memory_card: Option<PathBuf>,
        show_vram: bool,
        full_speed: bool,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            channels,
            shared_state,
            system: build_system(&bios_path, file_path.as_ref(), memory_card.as_deref())?,
            bios_path,
            file_path,
            memory_card,
            breakpoints: HashSet::new(),
            show_vram,
            full_speed,
        })
    }

    pub fn run(self) -> anyhow::Result<()> {
        let (prod, cons) = ringbuf::HeapRb::new(0x1000).split();
        let audio_stream = build_audio_stream(cons)?;

        std::thread::spawn(move || {
            info!("emulator thread started...");
            self.main_loop(&audio_stream, prod);
        });

        Ok(())
    }

    fn send_debug_snapshot(&self) {
        let _ = self.channels.snapshot_tx.try_send(self.system.snapshot());
    }

    fn save_memory_card_to_disk(&mut self) {
        let Some(path) = self.memory_card.as_ref() else {
            return;
        };

        let Some(card) = self.system.memory_card() else {
            return;
        };

        let Some(data) = card.dirty_data() else {
            return;
        };

        let tmp_path = path.with_extension("mcd.tmp");
        if let Err(err) =
            std::fs::write(&tmp_path, data).and_then(|()| std::fs::rename(&tmp_path, path))
        {
            tracing::error!("failed to save memory card: {err}");
        }
    }

    const fn update_core_gamepad(&mut self, new_state: &GamepadState) {
        let gamepad = self.system.gamepad_mut();
        gamepad.set_buttons(new_state.buttons);
        gamepad.set_analog_mode(new_state.analog_mode);
        gamepad.set_stick_axis(new_state.left_stick, new_state.right_stick);
    }

    fn send_frame_buffer(&self, buffer: FrameBuffer) {
        // Non-blocking send
        let _ = self.channels.frame_tx.try_send(buffer);
    }

    /// Process pending UI commands. Returns `true` if shutdown was requested.
    fn process_commands(&mut self) -> bool {
        while let Ok(command) = self.channels.ui_command_rx.try_recv() {
            match command {
                UiCommand::SetVramDisplay(show_vram) => self.show_vram = show_vram,
                UiCommand::Shutdown => return true,
                UiCommand::DebugRequestState => self.send_debug_snapshot(),
                UiCommand::SetSpeed(value) => self.full_speed = value,
                UiCommand::Restart => {
                    match build_system(
                        &self.bios_path,
                        self.file_path.as_ref(),
                        self.memory_card.as_deref(),
                    ) {
                        Ok(system) => {
                            info!("emulator thread restarted");
                            self.system = system;
                        }
                        Err(err) => error!(%err, "failed to restart emulator thread"),
                    }
                    self.shared_state.resume();
                }

                UiCommand::DebugSetBreakpoint(address, enabled) => {
                    if enabled {
                        self.breakpoints.insert(address);
                    } else {
                        self.breakpoints.remove(&address);
                    }
                }

                UiCommand::DebugStep => {
                    if !self.shared_state.is_paused() {
                        warn!("trying to step while emulator is unpaused");
                        continue;
                    }

                    self.system.step_instruction(self.show_vram);
                    self.send_debug_snapshot();
                }
            }
        }
        false
    }

    fn main_loop(mut self, audio_stream: &cpal::Stream, mut prod: HeapProd<i16>) {
        let mut last_paused = true;

        loop {
            if self.process_commands() {
                break;
            }

            if let Ok(new_state) = self.channels.input_rx.try_recv() {
                self.update_core_gamepad(&new_state);
            }

            let paused = self.shared_state.is_paused();
            if paused != last_paused {
                if paused {
                    audio_stream.pause().expect("pause audio stream");
                } else {
                    audio_stream.play().expect("play audio stream");
                }
                last_paused = paused;
            }

            if paused {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }

            let system = &mut self.system;

            if !self.breakpoints.is_empty() {
                system.run_till_breakpoint(&self.breakpoints, self.show_vram);
                self.send_debug_snapshot();
                self.shared_state.pause();
                continue;
            }

            system.run_frame(self.show_vram);

            // Blocking send
            if !self.full_speed {
                let mut audio = system.audio_samples.as_slice();

                while !audio.is_empty() {
                    let written = prod.push_slice(audio);

                    if written == 0 {
                        std::thread::sleep(Duration::from_micros(500));
                        continue;
                    }

                    audio = &audio[written..];
                }
            }

            // Try to save memory_card to disk at the same frequency
            if let Some(fb) = system.frame_buffer.take() {
                self.save_memory_card_to_disk();
                self.send_frame_buffer(fb);
            }
        }

        info!("emulator thread stopped!");
    }
}

fn build_audio_stream(mut cons: HeapCons<i16>) -> anyhow::Result<cpal::Stream> {
    let device = default_host()
        .default_output_device()
        .context("no output device available")?;

    let stream = device.build_output_stream(
        &AUDIO_STREAM_CONFIG,
        move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
            let written = cons.pop_slice(data);

            // Fill the rest with silence
            if written < data.len() {
                warn!("audio buffer underrun");
                data[written..].fill(0);
            }
        },
        move |err| error!("an error occurred on the output audio stream: {err}"),
        None,
    )?;

    Ok(stream)
}

#[derive(Default)]
pub struct SharedState {
    is_paused: AtomicBool,
}

impl SharedState {
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::Relaxed)
    }
}

pub fn parse_runnable(path: PathBuf) -> anyhow::Result<MediaPath> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("exe" | "ps-exe") => Ok(MediaPath::Exe(path)),
        Some("bin") => Ok(MediaPath::Bin(path)),
        Some("cue") => Ok(MediaPath::Cue(path)),
        _ => anyhow::bail!("unsupported file format"),
    }
}

fn build_system(
    bios_path: &Path,
    file_path: Option<&MediaPath>,
    memory_card: Option<&Path>,
) -> anyhow::Result<starpsx_core::System> {
    let bios: Box<[u8; 0x80000]> = std::fs::read(bios_path)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("bios is wrong size"))?;

    let mut builder = starpsx_core::PSXBuilder::new(bios);

    if let Some(path) = file_path {
        builder = builder.with_media(path.load()?);
    }

    if let Some(path) = memory_card {
        builder = builder.with_card(load_or_create_card(path)?);
    }

    builder.build()
}

fn load_or_create_card(path: &Path) -> anyhow::Result<Box<[u8; 0x20000]>> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let bytes = bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("memory card is wrong size"))?;

            info!(?path, "using existing memory card at");
            Ok(bytes)
        }

        Err(e) if e.kind() == ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let blank = Box::new(*include_bytes!("blank.mcd"));
            std::fs::write(path, blank.as_ref())?;

            info!(?path, "created new memory card at");
            Ok(blank)
        }

        Err(e) => Err(e.into()),
    }
}
