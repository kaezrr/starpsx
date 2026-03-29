use num_enum::FromPrimitive;

#[derive(Default)]
pub struct AdpcmHistory {
    old: i16,
    older: i16,
}

/// Decodes a given XA-ADPCM sector and returns the decoded audio samples
pub fn decode_sector(
    sector: &[u8],
    history: &mut [AdpcmHistory; 3],
    resamplers: &mut [CubicResampler; 3],
) -> Vec<i16> {
    let audio_header = AudioHeader(sector[0x13]);

    assert_eq!(audio_header.bits_per_channel(), BitsPerSample::Bit4);
    assert_ne!(audio_header.sample_rate(), SampleRate::Reserved);
    assert_ne!(audio_header.channel(), Channel::Reserved);

    let is_stereo = audio_header.channel() == Channel::Stereo;
    let is_18900 = audio_header.sample_rate() == SampleRate::R18900;

    // Ensure resamplers match the current sector's rate
    for r in resamplers.iter_mut() {
        let expected_step = if is_18900 { 3 } else { 6 };
        if r.step != expected_step {
            *r = CubicResampler::new(is_18900);
        }
    }

    // 18.9kHz sectors play for twice as long, so they generate twice the output samples
    let target_capacity = if is_18900 { 9408 } else { 4704 };
    let mut output_samples = Vec::with_capacity(target_capacity);

    for section in sector[0x18..0x92C].chunks_exact(128) {
        for blk in 0..4 {
            if is_stereo {
                let samples_l = decode_28_nibbles::<0>(section, blk, &mut history[0]);
                let samples_r = decode_28_nibbles::<1>(section, blk, &mut history[1]);

                for (sample_l, sample_r) in samples_l.into_iter().zip(samples_r) {
                    let (out_l, count_l) = resamplers[0].process_sample(sample_l);
                    let (out_r, _count_r) = resamplers[1].process_sample(sample_r);

                    (0..count_l).for_each(|i| {
                        output_samples.push(out_l[i]);
                        output_samples.push(out_r[i]);
                    });
                }
            } else {
                let samples_m1 = decode_28_nibbles::<0>(section, blk, &mut history[2]);
                let samples_m2 = decode_28_nibbles::<1>(section, blk, &mut history[2]);

                for sample in samples_m1.into_iter().chain(samples_m2) {
                    let (out, count) = resamplers[2].process_sample(sample);

                    (0..count).for_each(|i| {
                        output_samples.push(out[i]);
                    });
                }
            }
        }
    }

    output_samples
}

const POS_XA_ADPCM_TABLE: [i32; 5] = [0, 60, 115, 98, 122];
const NEG_XA_ADPCM_TABLE: [i32; 5] = [0, 0, -52, -55, -60];

#[must_use]
fn decode_28_nibbles<const NIBBLE: usize>(
    section: &[u8],
    blk: usize,
    history: &mut AdpcmHistory,
) -> [i16; 28] {
    let mut samples = [0; 28];

    let shift_raw = section[4 + blk * 2 + NIBBLE] & 0xF;
    let shift = 12 - if shift_raw > 12 { 9 } else { shift_raw };
    let filter = (section[4 + blk * 2 + NIBBLE] & 0x30) >> 4;

    let f0 = POS_XA_ADPCM_TABLE[usize::from(filter)];
    let f1 = NEG_XA_ADPCM_TABLE[usize::from(filter)];

    for i in 0..28 {
        let t = signed4bit((section[16 + blk + i * 4] >> (NIBBLE * 4)) & 0xF);
        let s = (t << shift)
            + ((i32::from(history.old) * f0 + i32::from(history.older) * f1 + 32) / 64);
        let s = s.clamp(-0x8000, 0x7FFF) as i16;

        history.older = history.old;
        history.old = s;

        samples[i] = s;
    }

    samples
}

bitfield::bitfield! {
    struct AudioHeader(u8);
    into Channel, channel, _ : 1, 0;
    into SampleRate, sample_rate, _ : 3, 2;
    into BitsPerSample, bits_per_channel, _ : 5, 4;
    emphasis, _ : 6;
}

#[derive(FromPrimitive, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Channel {
    #[default]
    Mono = 0,
    Stereo = 1,

    #[num_enum(alternatives = [3])]
    Reserved = 2,
}

#[derive(FromPrimitive, Debug, PartialEq, Eq)]
#[repr(u8)]
enum SampleRate {
    #[default]
    R37800 = 0,
    R18900 = 1,

    #[num_enum(alternatives = [3])]
    Reserved = 2,
}

#[derive(FromPrimitive, Debug, PartialEq, Eq)]
#[repr(u8)]
enum BitsPerSample {
    #[default]
    Bit4 = 0,
    Bit8 = 1,

    #[num_enum(alternatives = [3])]
    Reserved = 2,
}

fn signed4bit(v: u8) -> i32 {
    i32::from((v as i8) << 4 >> 4)
}

pub struct CubicResampler {
    history: [i16; 4],
    phase: usize,
    step: usize, // 3 for 18.9kHz, 6 for 37.8kHz
}

impl CubicResampler {
    pub const fn new(is_18900: bool) -> Self {
        Self {
            history: [0; 4],
            phase: 0,
            step: if is_18900 { 3 } else { 6 },
        }
    }

    /// Feeds one native sample and returns a buffer of interpolated 44.1kHz samples.
    /// Returns (buffer, count) because one input can produce 0, 1, or 2 outputs.
    #[allow(clippy::many_single_char_names)]
    pub fn process_sample(&mut self, sample: i16) -> ([i16; 3], usize) {
        self.history.copy_within(1..4, 0);
        self.history[3] = sample;

        let mut out_buffer = [0; 3];
        let mut count = 0;

        let y0 = f32::from(self.history[0]);
        let y1 = f32::from(self.history[1]);
        let y2 = f32::from(self.history[2]);
        let y3 = f32::from(self.history[3]);

        // Catmull-Rom coefficients
        let a = 0.5f32.mul_add(y3, 1.5f32.mul_add(-y2, (-0.5f32).mul_add(y0, 1.5 * y1)));
        let b = 0.5f32.mul_add(-y3, 2.0f32.mul_add(y2, 2.5f32.mul_add(-y1, y0)));
        let c = (-0.5f32).mul_add(y0, 0.5 * y2);
        let d = y1;

        while self.phase < 7 {
            let t = self.phase as f32 / 7.0;
            let res = (a * t + b).mul_add(t, c).mul_add(t, d);
            out_buffer[count] = res.clamp(-32768.0, 32767.0) as i16;
            count += 1;
            self.phase += self.step;
        }

        self.phase -= 7;
        (out_buffer, count)
    }
}
