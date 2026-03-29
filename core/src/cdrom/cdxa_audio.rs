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
    resamplers: &mut [ZigZagResampler; 3],
) -> Vec<i16> {
    let audio_header = AudioHeader(sector[0x13]);

    assert_eq!(audio_header.bits_per_channel(), BitsPerSample::Bit4);
    assert_ne!(audio_header.sample_rate(), SampleRate::Reserved);
    assert_ne!(audio_header.channel(), Channel::Reserved);

    let is_stereo = audio_header.channel() == Channel::Stereo;
    let mut output_samples = Vec::with_capacity(4704);

    // Audio data is in the 0x914 byte region.
    // It is split into 18 * 128 byte sized chunks = 0x900 bytes
    // Remaining 0x14 bytes are zero-filled
    for section in sector[0x18..0x92C].chunks_exact(128) {
        for blk in 0..4 {
            if is_stereo {
                let samples_l = decode_28_nibbles::<0>(section, blk, &mut history[0]);
                let samples_r = decode_28_nibbles::<1>(section, blk, &mut history[1]);

                for (sample_l, sample_r) in samples_l.into_iter().zip(samples_r) {
                    let resample_l = resamplers[0].process_sample(sample_l);
                    let resample_r = resamplers[1].process_sample(sample_r);

                    if let (Some(l), Some(r)) = (resample_l, resample_r) {
                        for i in 0..7 {
                            output_samples.push(l[i]);
                            output_samples.push(r[i]);
                        }
                    }
                }
            } else {
                let samples_mono1 = decode_28_nibbles::<0>(section, blk, &mut history[2]);
                let samples_mono2 = decode_28_nibbles::<1>(section, blk, &mut history[2]);

                for sample in samples_mono1 {
                    if let Some(s) = resamplers[2].process_sample(sample) {
                        output_samples.extend(s);
                    }
                }
                for sample in samples_mono2 {
                    if let Some(s) = resamplers[2].process_sample(sample) {
                        output_samples.extend(s);
                    }
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

pub struct ZigZagResampler {
    ringbuf: [i16; 32],
    p: usize,
    sixstep: usize,
}

impl Default for ZigZagResampler {
    fn default() -> Self {
        Self {
            ringbuf: [0; 32],
            p: 0,
            sixstep: 6,
        }
    }
}

impl ZigZagResampler {
    /// Resamples 37800Hz to 44100Hz by interpolating 7 samples for every 6 samples
    fn process_sample(&mut self, sample: i16) -> Option<[i16; 7]> {
        self.ringbuf[self.p & 0x1F] = sample;
        self.p = self.p.wrapping_add(1);
        self.sixstep -= 1;

        if self.sixstep == 0 {
            self.sixstep = 6;
            Some([
                self.interpolate(&XA_RESAMPLE_TABLES[0]),
                self.interpolate(&XA_RESAMPLE_TABLES[1]),
                self.interpolate(&XA_RESAMPLE_TABLES[2]),
                self.interpolate(&XA_RESAMPLE_TABLES[3]),
                self.interpolate(&XA_RESAMPLE_TABLES[4]),
                self.interpolate(&XA_RESAMPLE_TABLES[5]),
                self.interpolate(&XA_RESAMPLE_TABLES[6]),
            ])
        } else {
            None
        }
    }

    fn interpolate(&self, table: &[i32; 29]) -> i16 {
        let mut sum: i32 = 0;

        for i in 1..30 {
            let idx = self.p.wrapping_sub(i) & 0x1F;
            sum += (i32::from(self.ringbuf[idx]) * table[i - 1]) / 0x8000;
        }

        sum.clamp(-0x8000, 0x7FFF) as i16
    }
}

/// XA-ADPCM 25-point Zigzag Interpolation Tables
/// Each sub-array represents one of the 7 phases (Table 1 through Table 7)
const XA_RESAMPLE_TABLES: [[i32; 29]; 7] = [
    // Table 1
    [
        0x0000, 0x0000, 0x0000, 0x0000, 0x0000, -0x0002, 0x000A, -0x0022, 0x0041, -0x0054, 0x0034,
        0x0009, -0x010A, 0x0400, -0x0A78, 0x234C, 0x6794, -0x1780, 0x0BCD, -0x0623, 0x0350,
        -0x016D, 0x006B, 0x000A, -0x0010, 0x0011, -0x0008, 0x0003, -0x0001,
    ],
    // Table 2
    [
        0x0000, 0x0000, 0x0000, -0x0002, 0x0000, 0x0003, -0x0013, 0x003C, -0x004B, 0x00A2, -0x00E3,
        0x0132, -0x0043, -0x0267, 0x0C9D, 0x74BB, -0x11B4, 0x09B8, -0x05BF, 0x0372, -0x01A8,
        0x00A6, -0x001B, 0x0005, 0x0006, -0x0008, 0x0003, -0x0001, 0x0000,
    ],
    // Table 3
    [
        0x0000, 0x0000, -0x0001, 0x0003, -0x0002, -0x0005, 0x001F, -0x004A, 0x00B3, -0x0192,
        0x02B1, -0x039E, 0x04F8, -0x05A6, 0x7939, -0x05A6, 0x04F8, -0x039E, 0x02B1, -0x0192,
        0x00B3, -0x004A, 0x001F, -0x0005, -0x0002, 0x0003, -0x0001, 0x0000, 0x0000,
    ],
    // Table 4
    [
        0x0000, -0x0001, 0x0003, -0x0008, 0x0006, 0x0005, -0x001B, 0x00A6, -0x01A8, 0x0372,
        -0x05BF, 0x09B8, -0x11B4, 0x74BB, 0x0C9D, -0x0267, -0x0043, 0x0132, -0x00E3, 0x00A2,
        -0x004B, 0x003C, -0x001B, 0x0003, 0x0000, -0x0002, 0x0000, 0x0000, 0x0000,
    ],
    // Table 5
    [
        -0x0001, 0x0003, -0x0008, 0x0011, -0x0010, 0x000A, 0x006B, -0x016D, 0x0350, -0x0623,
        0x0BCD, -0x1780, 0x6794, 0x234C, -0x0A78, 0x0400, -0x010A, 0x0009, 0x0034, -0x0054, 0x0041,
        -0x0022, 0x000A, -0x0001, 0x0000, 0x0001, 0x0000, 0x0000, 0x0000,
    ],
    // Table 6
    [
        0x0002, -0x0008, 0x0010, -0x0023, 0x002B, 0x001A, -0x00EB, 0x027B, -0x0548, 0x0AFA,
        -0x16FA, 0x53E0, 0x3C07, -0x1249, 0x080E, -0x0347, 0x015B, -0x0044, -0x0017, 0x0046,
        -0x0023, 0x0011, -0x0005, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
    // Table 7
    [
        -0x0005, 0x0011, -0x0023, 0x0046, -0x0017, -0x0044, 0x015B, -0x0347, 0x080E, -0x1249,
        0x3C07, 0x53E0, -0x16FA, 0x0AFA, -0x0548, 0x027B, -0x00EB, 0x001A, 0x002B, -0x0023, 0x0010,
        -0x0008, 0x0002, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    ],
];
