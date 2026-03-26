use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use cpal::StreamConfig;
use cpal::default_host;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use starpsx_core::RunType;
use starpsx_core::SystemSnapshot;
use starpsx_renderer::FrameBuffer;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::config::RunnablePath;
use crate::input::GamepadState;

const FRAME_TIME: Duration = Duration::from_nanos(16_666_667);

const AUDIO_STREAM_CONFIG: StreamConfig = StreamConfig {
    channels: 2,
    sample_rate: 44100_u32,
    buffer_size: cpal::BufferSize::Default,
};

pub enum UiCommand {
    NewInputState(GamepadState),
    SetVramDisplay(bool),
    SetSpeed(bool),
    Restart,
    Shutdown,

    DebugSetBreakpoint(u32, bool),
    DebugStep,
    DebugRequestState,
}

pub struct UiChannels {
    pub frame_tx: SyncSender<FrameBuffer>,
    pub input_rx: Receiver<UiCommand>,
    pub snapshot_tx: SyncSender<SystemSnapshot>,
}

pub struct Emulator {
    channels: UiChannels,
    shared_state: Arc<SharedState>,
    system: starpsx_core::System,
    breakpoints: HashSet<u32>,
    bios_path: PathBuf,
    file_path: Option<RunnablePath>,
    memory_card: Option<PathBuf>,
    show_vram: bool,
    full_speed: bool,
}

impl Emulator {
    pub fn build(
        channels: UiChannels,
        shared_state: Arc<SharedState>,
        bios_path: PathBuf,
        file_path: Option<RunnablePath>,
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
        let (audio_tx, audio_rx) = mpsc::sync_channel::<[i16; 2]>(8192);
        let audio_stream = build_audio_stream(audio_rx)?;

        std::thread::spawn(move || {
            info!("emulator thread started...");
            self.main_loop(&audio_stream, &audio_tx);
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
        while let Ok(command) = self.channels.input_rx.try_recv() {
            match command {
                UiCommand::SetVramDisplay(show_vram) => self.show_vram = show_vram,
                UiCommand::Shutdown => return true,
                UiCommand::DebugRequestState => self.send_debug_snapshot(),
                UiCommand::NewInputState(state) => self.update_core_gamepad(&state),
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

                    if let Some(fb) = self.system.step_instruction(self.show_vram) {
                        self.send_frame_buffer(fb);
                    }
                    self.send_debug_snapshot();
                }
            }
        }
        false
    }

    fn main_loop(mut self, audio_stream: &cpal::Stream, audio_rx: &SyncSender<[i16; 2]>) {
        let mut last_paused = true;

        loop {
            if self.process_commands() {
                break;
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
                std::thread::sleep(Duration::from_millis(16));
                continue;
            }

            let frame_start = Instant::now();

            let frame = if self.breakpoints.is_empty() {
                let fb = self.system.run_frame(self.show_vram);
                self.save_memory_card_to_disk();
                Some(fb)
            } else {
                self.system
                    .run_breakpoint(&self.breakpoints, self.show_vram)
            };

            let core_time = frame_start.elapsed();

            if let Some(buffer) = frame {
                self.send_frame_buffer(buffer);
            } else {
                self.shared_state.pause();
                self.send_debug_snapshot();
                continue;
            }

            if !self.full_speed
                && let Some(sleep_dur) = FRAME_TIME.checked_sub(core_time)
            {
                spin_sleep::sleep(sleep_dur);
            }

            let total_time = frame_start.elapsed();
            self.shared_state
                .store(total_time.as_secs_f32(), core_time.as_secs_f32());
        }

        info!("emulator thread stopped!");
    }
}

fn build_audio_stream(audio_rx: Receiver<[i16; 2]>) -> anyhow::Result<cpal::Stream> {
    let device = default_host()
        .default_output_device()
        .context("no output device available")?;

    let stream = device.build_output_stream(
        &AUDIO_STREAM_CONFIG,
        move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_exact_mut(2) {
                let frame_out = audio_rx.recv().expect("recv on audio channel");
                frame.copy_from_slice(&frame_out);
            }
        },
        move |err| {
            error!("an error occurred on the output audio stream: {err}");
        },
        None,
    )?;

    Ok(stream)
}

#[derive(Default)]
pub struct SharedState {
    frame_time_ms: AtomicU32,
    core_time_ms: AtomicU32,
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

    pub fn store(&self, frame_time_ms: f32, core_time_ms: f32) {
        self.frame_time_ms
            .store(frame_time_ms.to_bits(), Ordering::Relaxed);
        self.core_time_ms
            .store(core_time_ms.to_bits(), Ordering::Relaxed);
    }

    pub fn load(&self) -> (f32, f32) {
        let fps = f32::from_bits(self.frame_time_ms.load(Ordering::Relaxed));
        let core_time_ms = f32::from_bits(self.core_time_ms.load(Ordering::Relaxed));
        (fps, core_time_ms)
    }
}

pub fn parse_runnable(path: PathBuf) -> anyhow::Result<RunnablePath> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("exe" | "ps-exe") => Ok(RunnablePath::Exe(path)),
        Some("bin") => Ok(RunnablePath::Bin(path)),
        Some("cue") => Ok(RunnablePath::Cue(path)),
        _ => anyhow::bail!("unsupported file format"),
    }
}

fn build_system(
    bios_path: &Path,
    file_path: Option<&RunnablePath>,
    memory_card: Option<&Path>,
) -> anyhow::Result<starpsx_core::System> {
    let bios = std::fs::read(bios_path)?;
    let run_type = file_path
        .map(|run_type| -> anyhow::Result<RunType> {
            let bytes = match run_type {
                RunnablePath::Exe(path) => RunType::Executable(std::fs::read(path)?),
                RunnablePath::Bin(path) => RunType::Binary(std::fs::read(path)?),
                RunnablePath::Cue(path) => RunType::Disk(cue::build_disk(path)?),
            };
            Ok(bytes)
        })
        .transpose()?;

    let memory_card = memory_card
        .as_ref()
        .map(|path| -> anyhow::Result<Box<[u8; 0x20000]>> {
            let bytes = if path.exists() {
                let bytes = std::fs::read(path)?;
                bytes
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("memory card is wrong size"))?
            } else {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let blank = Box::new(*include_bytes!("blank.mcd"));
                std::fs::write(path, blank.as_ref())?;
                blank
            };
            info!(?path, "Using memory card");
            Ok(bytes)
        })
        .transpose()?;

    let system = starpsx_core::System::build(bios, run_type, memory_card)?;
    Ok(system)
}
