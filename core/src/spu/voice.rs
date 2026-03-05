use super::adsr::AdsrEnvelope;
use super::*;

#[derive(Default)]
pub struct Voice {
    pub volume: Volume,
    pub sample_rate: u16,

    pub start_address: u16,
    pub repeat_address: u16,
    pub current_address: usize,

    pub pitch_counter: u16,
    pub envelope: AdsrEnvelope,

    decode_buffer: [i16; 28],
    current_buffer_idx: usize,

    adpcm_older_sample: i16,
    adpcm_old_sample: i16,

    oldest_sample: i16,
    older_sample: i16,
    old_sample: i16,
    current_sample: i16,
}

impl Voice {
    pub fn key_on(&mut self, sound_ram: &[u8]) {
        self.envelope.key_on();

        self.current_address = (self.start_address << 3) as usize;
        self.pitch_counter = 0;
        self.decode_next_block(sound_ram)
    }

    pub fn key_off(&mut self) {
        self.envelope.key_off();
    }

    pub fn tick(&mut self, sound_ram: &[u8]) -> (i16, i16) {
        let pitch_counter_step = self.sample_rate.min(0x4000);
        self.pitch_counter += pitch_counter_step;

        while self.pitch_counter >= 0x1000 {
            self.pitch_counter -= 0x1000;
            self.current_buffer_idx += 1;

            if self.current_buffer_idx == 28 {
                self.current_buffer_idx = 0;
                self.decode_next_block(sound_ram);
            }
        }

        // Shift the 4-sample window forward
        self.oldest_sample = self.older_sample;
        self.older_sample = self.old_sample;
        self.old_sample = self.current_sample;
        self.current_sample = self.decode_buffer[self.current_buffer_idx];

        // i = bit 4-11 of the pitch counter (8-bit index)
        let i = ((self.pitch_counter >> 4) & 0xFF) as usize;

        // Apply the Gaussian interpolation
        let mut interpolated: i32;
        interpolated = (GAUSSIAN_TABLE[0x0FF - i] * self.oldest_sample as i32) >> 15;
        interpolated += (GAUSSIAN_TABLE[0x1FF - i] * self.older_sample as i32) >> 15;
        interpolated += (GAUSSIAN_TABLE[0x100 + i] * self.old_sample as i32) >> 15;
        interpolated += (GAUSSIAN_TABLE[i] * self.current_sample as i32) >> 15;

        self.envelope.tick();
        self.apply_voice_volume(interpolated as i16)
    }

    fn decode_next_block(&mut self, sound_ram: &[u8]) {
        let block = &sound_ram[self.current_address..self.current_address + 16]
            .try_into()
            .unwrap();

        self.decode_adpcm_block(block);

        let loop_end = block[1] & 1 != 0;
        let loop_repeat = block[1] & (1 << 1) != 0;
        let loop_start = block[1] & (1 << 2) != 0;

        if loop_start {
            self.repeat_address = (self.current_address >> 3) as u16;
        }

        if loop_end {
            self.current_address = (self.repeat_address << 3) as usize;

            if !loop_repeat {
                self.envelope.level = 0;
                self.envelope.key_off();
            }
        } else {
            self.current_address += 16;
        }
    }

    fn decode_adpcm_block(&mut self, block: &[u8; 16]) {
        let shift = block[0] & 0x0F;
        let shift = if shift > 12 { 9 } else { shift };

        let filter = ((block[0] >> 4) & 0x07).min(4);

        for sample_idx in 0..28 {
            let sample_byte = block[2 + sample_idx / 2];
            let sample_nibble = (sample_byte >> (4 * (sample_idx % 2))) & 0x0F;

            let raw_sample: i32 = (((sample_nibble as i8) << 4) >> 4).into();

            let shifted_sample = raw_sample << (12 - shift);

            let old = i32::from(self.adpcm_old_sample);
            let older = i32::from(self.adpcm_older_sample);
            let filtered_sample = match filter {
                0 => shifted_sample,
                1 => shifted_sample + (60 * old + 32) / 64,
                2 => shifted_sample + (115 * old - 52 * older + 32) / 64,
                3 => shifted_sample + (98 * old - 55 * older + 32) / 64,
                4 => shifted_sample + (122 * old - 60 * older + 32) / 64,
                _ => unreachable!("filter was clamped to 0..=4"),
            };

            let clamped_sample = filtered_sample.clamp(-0x8000, 0x7FFF) as i16;
            self.decode_buffer[sample_idx] = clamped_sample;

            self.adpcm_older_sample = self.adpcm_old_sample;
            self.adpcm_old_sample = clamped_sample;
        }
    }

    fn apply_voice_volume(&self, adpcm_sample: i16) -> (i16, i16) {
        let envelope_sample = apply_volume(adpcm_sample, self.envelope.level);

        let output_l = apply_volume(envelope_sample, self.volume.l);
        let output_r = apply_volume(envelope_sample, self.volume.r);

        (output_l, output_r)
    }
}
