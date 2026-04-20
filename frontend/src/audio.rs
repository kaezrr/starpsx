use anyhow::Context;
use cpal::StreamConfig;
use cpal::default_host;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use crossbeam::channel::Sender;
use tracing::error;

pub type AudioSample = [i16; 2];

const AUDIO_STREAM_CONFIG: StreamConfig = StreamConfig {
    channels: 2,
    sample_rate: 44100_u32,
    buffer_size: cpal::BufferSize::Default,
};

pub fn build_audio_stream() -> anyhow::Result<(cpal::Stream, Sender<AudioSample>)> {
    let (prod, cons) = crossbeam::channel::bounded(735);

    let device = default_host()
        .default_output_device()
        .context("no output device available")?;

    let stream = device.build_output_stream(
        &AUDIO_STREAM_CONFIG,
        move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
            for d in data.as_chunks_mut::<2>().0 {
                *d = cons.recv().unwrap_or_default();
            }
        },
        move |err| error!("an error occurred on the output audio stream: {err}"),
        None,
    )?;

    Ok((stream, prod))
}
