use crate::consts::NEG_ADPCM_TABLE;
use crate::consts::POS_ADPCM_TABLE;
use crate::spu::Sweep;
use crate::spu::Volume;

#[derive(Default)]
pub struct Voice {
    pub volume: Volume<Sweep>,
    pub sample_rate: u16,

    /// This register holds the sample start address (not the current address, ie. the register doesn't increment during playback).
    /// Writing to this register has no effect on the currently playing voice.
    /// The start address is copied to the current address upon Key On.
    pub start_address: u32,

    /// If the hardware finds an ADPCM header with Loop-Start-Bit,
    /// then it copies the current address to the repeat addresss register.
    /// If the hardware finds an ADPCM header with Loop-Stop-Bit,
    /// then it copies the repeat addresss register setting to the current address; after playing the current ADPCM block.
    pub repeat_address: u32,

    pub current_address: u32,
    pub keyed_off: bool,

    decode_buffer: [i16; 28],
    current_buffer_idx: usize,

    adpcm_old_sample: i16,
    adpcm_older_sample: i16,

    force_loop_index: bool,
    pitch_counter: u16,

    pub current_sample: i16,
}

impl Voice {
    /// (0=stop, 4000h=fastest, 4001h..FFFFh=usually same as 4000h)
    pub const fn set_sample_rate(&mut self, val: u16) {
        self.sample_rate = val;
    }

    pub fn set_start_address(&mut self, val: u16) {
        self.start_address = u32::from(val) * 8;
    }

    pub fn set_repeat_address(&mut self, val: u16) {
        self.repeat_address = u32::from(val) * 8;
        self.force_loop_index = true;
    }

    pub fn key_on(&mut self, sound_ram: &[u8]) {
        // envelope key on

        self.current_address = self.start_address;
        self.force_loop_index = false;
        self.keyed_off = false;

        self.decode_next_block(sound_ram);
    }

    pub const fn key_off(&mut self) {
        self.keyed_off = true;
    }

    pub fn tick(&mut self, sound_ram: &[u8]) {
        let pitch_counter_step = self.sample_rate.min(0x4000);
        // Do pitch modulation here

        self.pitch_counter += pitch_counter_step;

        while self.pitch_counter >= 0x1000 {
            self.pitch_counter -= 0x1000;
            self.current_buffer_idx += 1;

            if self.current_buffer_idx == 28 {
                self.current_buffer_idx = 0;
                self.decode_next_block(sound_ram);
            }
        }

        // Sample interpolation and voice volume here
        self.current_sample = self.decode_buffer[self.current_buffer_idx];
    }

    fn decode_next_block(&mut self, sound_ram: &[u8]) {
        let addr = self.current_address as usize;
        let block = &sound_ram[addr..addr + 16];
        self.decode_adpcm_block(block);

        let loop_end = block[1] & 1 != 0;
        let loop_repeat = block[1] & 2 != 0;
        let loop_start = block[1] & 4 != 0;

        if loop_start && !self.force_loop_index {
            self.repeat_address = self.current_address;
        }

        if loop_end {
            self.current_address = self.repeat_address;

            if loop_repeat {
                // Envelope key off
            }
        } else {
            self.current_address += 16;
        }
    }

    fn decode_adpcm_block(&mut self, block: &[u8]) {
        let shift = block[0] & 0x0F;
        let shift = 12 - if shift > 12 { 9 } else { shift };
        let filter = (block[0] & 0x70) >> 4;

        let f0 = POS_ADPCM_TABLE[usize::from(filter)];
        let f1 = NEG_ADPCM_TABLE[usize::from(filter)];

        for i in 0..28 {
            let old = i32::from(self.adpcm_old_sample);
            let older = i32::from(self.adpcm_older_sample);

            let t = super::signed4bit((block[2 + i / 2] >> (4 * (i & 1))) & 0xF);
            let s = (t << shift) + (old * f0 + older * f1 + 32) / 64;
            let s = s.clamp(-0x8000, 0x7FFF) as i16;

            self.adpcm_older_sample = self.adpcm_old_sample;
            self.adpcm_old_sample = s;

            self.decode_buffer[i] = s;
        }
    }
}
