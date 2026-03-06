use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::time::{Duration, Instant};

use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig, default_host};
use eframe::egui;
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Split};
use starpsx_core::RunType;
use starpsx_renderer::FrameBuffer;
use tracing::{error, info, warn};

use crate::config::RunnablePath;
use crate::input::GamepadState;
use starpsx_core::SystemSnapshot;

const RING_BUFFER_SIZE: usize = starpsx_core::AUDIO_CHUNK_SIZE * 8;

pub enum UiCommand {
    NewInputState(GamepadState),
    SetVramDisplay(bool),
    Shutdown,
    Restart,

    DebugSetBreakpoint(u32, bool),
    DebugStep,
    DebugRequestState,
}

pub struct Emulator {
    ui_ctx: egui::Context,
    channels: UiChannels,
    shared_state: Arc<SharedState>,
    system: starpsx_core::System,
    audio_stream: Stream,
    breakpoints: HashSet<u32>,
    show_vram: bool,
    bios_path: PathBuf,
    file_path: Option<RunnablePath>,
}

pub struct UiChannels {
    pub frame_tx: SyncSender<FrameBuffer>,
    pub input_rx: Receiver<UiCommand>,
    pub snapshot_tx: SyncSender<SystemSnapshot>,
}

impl Emulator {
    pub fn build(
        ui_ctx: egui::Context,

        channels: UiChannels,
        shared_state: Arc<SharedState>,

        bios_path: PathBuf,
        file_path: Option<RunnablePath>,

        show_vram: bool,
    ) -> anyhow::Result<Self> {
        let (system, audio_stream) = Self::build_system(&bios_path, file_path.as_ref())?;
        let breakpoints = HashSet::new();

        Ok(Self {
            ui_ctx,
            channels,
            shared_state,
            system,
            audio_stream,
            breakpoints,
            show_vram,
            bios_path,
            file_path,
        })
    }

    fn build_system(
        bios_path: &PathBuf,
        file_path: Option<&RunnablePath>,
    ) -> anyhow::Result<(starpsx_core::System, Stream)> {
        const STREAM_CONFIG: StreamConfig = StreamConfig {
            channels: 2,
            sample_rate: 44100,
            buffer_size: cpal::BufferSize::Default,
        };

        let device = default_host()
            .default_output_device()
            .ok_or(anyhow!("no output device available"))?;

        let rb = HeapRb::<[i16; 2]>::new(RING_BUFFER_SIZE);
        let (audio_tx, mut audio_rx) = rb.split();

        let audio_stream = device.build_output_stream(
            &STREAM_CONFIG,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                let mut chunks = data.chunks_exact_mut(2);

                for frame in &mut chunks {
                    let frame_out = audio_rx.try_pop().unwrap_or([0, 0]);
                    frame.copy_from_slice(&frame_out);
                }

                for sample in chunks.into_remainder() {
                    warn!("audio samples len is odd");
                    *sample = 0;
                }
            },
            move |err| {
                error!("an error occurred on the output audio stream: {err}");
            },
            None,
        )?;

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

        let system = starpsx_core::System::build(bios, run_type, audio_tx)?;
        Ok((system, audio_stream))
    }

    fn rebuild_system(&mut self) -> anyhow::Result<()> {
        let (system, audio_stream) = Self::build_system(&self.bios_path, self.file_path.as_ref())?;
        self.system = system;
        self.audio_stream = audio_stream;
        Ok(())
    }

    pub fn run(self) {
        std::thread::spawn(move || self.main_loop());
        info!("emulator thread started...");
    }

    fn send_debug_snapshot(&self) {
        let _ = self.channels.snapshot_tx.try_send(self.system.snapshot());
        self.ui_ctx.request_repaint();
    }

    fn update_core_gamepad(&mut self, new_state: GamepadState) {
        let gamepad = self.system.gamepad_mut();
        gamepad.set_buttons(new_state.buttons);
        gamepad.set_analog_mode(new_state.analog_mode);
        gamepad.set_stick_axis(new_state.left_stick, new_state.right_stick);
    }

    fn send_frame_buffer(&mut self, buffer: FrameBuffer) {
        // Non blocking send
        let _ = self.channels.frame_tx.try_send(buffer);
        self.ui_ctx.request_repaint();
    }

    fn main_loop(mut self) {
        const FRAME_TIME: Duration = Duration::from_nanos(16_666_667);

        let mut last_paused = true;

        'emulator: loop {
            // Read events from ui thread
            while let Ok(command) = self.channels.input_rx.try_recv() {
                match command {
                    UiCommand::NewInputState(gamepad_state) => {
                        self.update_core_gamepad(gamepad_state)
                    }

                    UiCommand::SetVramDisplay(show_vram) => {
                        self.show_vram = show_vram;
                    }

                    UiCommand::Shutdown => {
                        break 'emulator;
                    }

                    // should restart the emulator with the same bios and game (if any).
                    // should do this by just rebuild the system and replacing it.
                    UiCommand::Restart => match self.rebuild_system() {
                        Ok(()) => info!("emulator restarted"),
                        Err(e) => error!("failed to restart emulator: {e}"),
                    },

                    UiCommand::DebugRequestState => {
                        self.send_debug_snapshot();
                    }

                    UiCommand::DebugSetBreakpoint(address, enabled) => {
                        match enabled {
                            true => self.breakpoints.insert(address),
                            false => self.breakpoints.remove(&address),
                        };
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

            let paused = self.shared_state.is_paused();

            if paused != last_paused {
                if paused {
                    self.audio_stream.pause().unwrap();
                } else {
                    self.audio_stream.play().unwrap();
                }
                last_paused = paused;
            }

            if paused {
                std::thread::sleep(Duration::from_millis(16));
                continue;
            }

            let now = Instant::now();
            let vram = self.show_vram;

            let frame_opt = if self.breakpoints.is_empty() {
                Some(self.system.run_frame(vram))
            } else {
                self.system.run_breakpoint(&self.breakpoints, vram)
            };

            let core_time = now.elapsed();

            if let Some(buffer) = frame_opt {
                self.send_frame_buffer(buffer);
            } else {
                self.shared_state.pause();
                self.send_debug_snapshot();
                continue;
            }

            if let Some(sleep_dur) = FRAME_TIME.checked_sub(core_time) {
                std::thread::sleep(sleep_dur);
            }

            let core_time = core_time.as_secs_f32();
            let frame_time = now.elapsed().as_secs_f32();

            self.shared_state.store(frame_time, core_time);
        }
        info!("emulator thread stopped!");
    }
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
        Some("exe") | Some("ps-exe") => Ok(RunnablePath::Exe(path)),
        Some("bin") => Ok(RunnablePath::Bin(path)),
        Some("cue") => Ok(RunnablePath::Cue(path)),
        _ => Err(anyhow!("unsupported file format")),
    }
}

use std::fs::File;
use std::io::Write;

#[expect(unused)]
fn write_wav(samples: &[[i16; 2]]) -> std::io::Result<()> {
    let mut file = File::create("./stuff/test.wav")?;

    let channels = 2u16;
    let bits_per_sample = 16u16;
    let sample_rate = 44100;

    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;

    let data_size = (samples.len() * 4) as u32;
    let file_size = 36 + data_size;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?; // PCM
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;

    for [l, r] in samples {
        file.write_all(&l.to_le_bytes())?;
        file.write_all(&r.to_le_bytes())?;
    }

    Ok(())
}
