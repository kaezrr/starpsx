use std::error::Error;
use std::sync::mpsc::Receiver;

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Sample, Stream, StreamConfig};
use tracing::{error, info, warn};

pub fn build(audio_rx: Receiver<[i16; 2]>) -> Result<(Stream, u32), Box<dyn Error>> {
    let audio_device = cpal::default_host()
        .default_output_device()
        .ok_or("no audio output device available")?;

    let mut supported_config_range = audio_device
        .supported_output_configs()
        .map_err(|_| "error while querying audio configs")?;

    let default_config = audio_device.default_output_config()?;

    let supported_config = supported_config_range
        .find(|c| {
            let min = c.min_sample_rate();
            let max = c.max_sample_rate();
            // Need stereo audio configuration that runs at 44.1Khz
            // as the emulator is synced to audio
            min <= 44100 && 44100 <= max && c.channels() == 2
        })
        .map(|c| c.with_sample_rate(44100))
        .unwrap_or_else(|| {
            error!("could not find 44.1Khz audio config, falling back to default");
            error!("as a consequence, games will run faster than intended");
            default_config
        });

    let config = supported_config.config();

    info!(?config, "using audio configuration");

    let stream = match supported_config.sample_format() {
        cpal::SampleFormat::I16 => build_stream::<i16>(&audio_device, &config, audio_rx),
        cpal::SampleFormat::F32 => build_stream::<f32>(&audio_device, &config, audio_rx),
        cpal::SampleFormat::I8 => build_stream::<i8>(&audio_device, &config, audio_rx),
        cpal::SampleFormat::I32 => build_stream::<i32>(&audio_device, &config, audio_rx),
        cpal::SampleFormat::I64 => build_stream::<i64>(&audio_device, &config, audio_rx),
        cpal::SampleFormat::U8 => build_stream::<u8>(&audio_device, &config, audio_rx),
        cpal::SampleFormat::U16 => build_stream::<u16>(&audio_device, &config, audio_rx),
        cpal::SampleFormat::U32 => build_stream::<u32>(&audio_device, &config, audio_rx),
        cpal::SampleFormat::U64 => build_stream::<u64>(&audio_device, &config, audio_rx),
        cpal::SampleFormat::F64 => build_stream::<f64>(&audio_device, &config, audio_rx),
        sample_format => unreachable!("unsupported sample format {sample_format}"),
    }?;

    Ok((stream, config.sample_rate))
}

fn build_stream<T: Sample + cpal::FromSample<i16> + cpal::SizedSample>(
    device: &Device,
    config: &StreamConfig,
    sample_rx: Receiver<[i16; 2]>,
) -> Result<Stream, cpal::BuildStreamError> {
    device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_exact_mut(2) {
                let received = sample_rx.recv().unwrap_or_else(|err| {
                    warn!(%err,"audio channel error");
                    [0, 0]
                });

                frame[0] = received[0].to_sample();
                frame[1] = received[1].to_sample();
            }
        },
        move |err| warn!(%err, "audio stream error"),
        None,
    )
}
