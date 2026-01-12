#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod util;

use std::error::Error;
use std::sync::mpsc::{Receiver, SyncSender};

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Sample, Stream, StreamConfig};
use eframe::egui;
use tracing::error;
use tracing_subscriber::fmt;

use starpsx_renderer::FrameBuffer;
use util::GamepadState;

fn main() -> eframe::Result {
    fmt().without_time().init();

    let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel::<FrameBuffer>(1);
    let (input_tx, input_rx) = std::sync::mpsc::sync_channel::<GamepadState>(1);
    let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<i16>(10);

    let config = starpsx_core::Config::build().unwrap_or_else(|err| {
        error!(%err, "Failed to parse command-line arguments");
        std::process::exit(1);
    });

    let system = starpsx_core::System::build(config).unwrap_or_else(|err| {
        error!(%err, "Error while starting emulator");
        std::process::exit(1);
    });

    let stream = build_audio(audio_rx).unwrap_or_else(|err| {
        error!(?err, "error while building audio stream");
        std::process::exit(1);
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "StarPSX",
        options,
        // This must only be called once!
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || run_core(ctx, frame_tx, input_rx, audio_tx, system));
            Ok(Box::new(app::Application::new(
                cc, frame_rx, input_tx, stream,
            )))
        }),
    )
}

fn run_core(
    repaint_notifier: egui::Context,
    frame_tx: SyncSender<FrameBuffer>,
    input_rx: Receiver<GamepadState>,
    audio_tx: SyncSender<i16>,
    mut system: starpsx_core::System,
) {
    loop {
        while let Ok(input_state) = input_rx.try_recv() {
            let gamepad = system.gamepad_mut();
            gamepad.set_buttons(input_state.buttons);
            gamepad.set_analog_mode(input_state.analog_mode);
            gamepad.set_stick_axis(input_state.left_stick, input_state.right_stick);
        }

        if let Some(sample) = system.tick() {
            audio_tx.send(sample).unwrap_or_else(|err| {
                error!(%err, "could not send sample to audio thread, exiting...");
                std::process::exit(1);
            })
        }

        let frame_sent = system
            .produced_frame_buffer
            .take()
            .map(|buf| frame_tx.try_send(buf).is_ok())
            .unwrap_or(false);

        if frame_sent {
            repaint_notifier.request_repaint();
        };
    }
}

fn build_audio(audio_rx: Receiver<i16>) -> Result<Stream, Box<dyn Error>> {
    let audio_device = cpal::default_host()
        .default_output_device()
        .ok_or("no audio output device available")?;

    let mut supported_config_range = audio_device
        .supported_output_configs()
        .map_err(|_| "error while querying audio configs")?;

    let supported_config = supported_config_range
        .find(|c| {
            matches!(
                c.sample_format(),
                cpal::SampleFormat::I16 | cpal::SampleFormat::F32
            )
        })
        .ok_or("no suitable audio config found")?
        .with_sample_rate(44100); // 44.1KHz

    let sample_format = supported_config.sample_format();
    let config = supported_config.into();

    match sample_format {
        cpal::SampleFormat::I16 => build_stream::<i16>(&audio_device, &config, audio_rx),
        cpal::SampleFormat::F32 => build_stream::<f32>(&audio_device, &config, audio_rx),
        sample_format => unreachable!("unsupported sample format {sample_format}"),
    }
    .map_err(|err| err.into())
}

fn build_stream<T: Sample + cpal::FromSample<i16> + cpal::SizedSample>(
    device: &Device,
    config: &StreamConfig,
    sample_rx: Receiver<i16>,
) -> Result<Stream, cpal::BuildStreamError> {
    device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for sample in data.iter_mut() {
                *sample = sample_rx.recv().unwrap().to_sample();
            }
        },
        move |err| error!(%err, "audio stream error"),
        None,
    )
}
