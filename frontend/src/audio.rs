use std::error::Error;
use std::sync::mpsc::Receiver;

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Sample, Stream, StreamConfig};
use tracing::{info, warn};

pub fn build(audio_rx: Receiver<i16>) -> Result<Stream, Box<dyn Error>> {
    let audio_device = cpal::default_host()
        .default_output_device()
        .ok_or("no audio output device available")?;

    let mut supported_config_range = audio_device
        .supported_output_configs()
        .map_err(|_| "error while querying audio configs")?;

    let supported_config = supported_config_range
        .find(|c| {
            c.channels() == 2 // Stereo
                && matches!(
                    c.sample_format(),
                    cpal::SampleFormat::I16 | cpal::SampleFormat::F32
                )
        })
        .ok_or("no suitable audio config found")?
        .with_sample_rate(44100); // 44.1KHz

    let sample_format = supported_config.sample_format();
    let config = supported_config.into();

    info!(?config, "using audio configuration");

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
        move |err| warn!(%err, "audio stream error"),
        None,
    )
}
