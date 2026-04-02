mod envelope;
mod utils;
mod voice;

use num_enum::FromPrimitive;
use num_enum::IntoPrimitive;
use tracing::trace;
use tracing::warn;
use utils::GAUSSIAN_TABLE;
pub use utils::signed4bit;
use utils::write_half;
use voice::Voice;

use crate::System;
use crate::spu::envelope::SweepVolumeLR;

pub const PADDR_START: u32 = 0x1F80_1C00;
pub const PADDR_END: u32 = 0x1F80_1E80;

bitfield::bitfield! {
    struct Control(u16);
    enabled, _ : 15;
    unmuted, _ : 14;
    u8, from into RamMode, ram_mode, _: 5, 4;
}

#[derive(FromPrimitive, IntoPrimitive, Debug)]
#[repr(u8)]
enum RamMode {
    #[default]
    Stop = 0,
    ManualWrite = 1,
    DmaWrite = 2,
    DmaRead = 3,
}

pub struct Spu {
    main_volume: SweepVolume,
    cd_volume: Volume,
    external_volume: Volume,

    voice_key_off: u32,
    voice_key_on: u32,
    voice_pitch_enable: u32,
    voice_noise_enable: u32,
    voice_echo_on: u32,

    voices: [Voice; 24],

    data_transfer_control: u16,
    data_transfer_address: u16,
    current_address: usize,

    control: Control,
    sound_irq_address: usize,

    sound_ram: Box<[u8; 0x80000]>,
}

impl Default for Spu {
    fn default() -> Self {
        Self {
            main_volume: SweepVolume::default(),
            cd_volume: Volume::default(),
            external_volume: Volume::default(),

            voice_key_off: 0,
            voice_key_on: 0,
            voice_pitch_enable: 0,
            voice_noise_enable: 0,
            voice_echo_on: 0,

            voices: std::array::from_fn(|_| Voice::default()),

            data_transfer_control: 0,
            data_transfer_address: 0,
            current_address: 0,

            control: Control(0),
            sound_irq_address: 0,

            sound_ram: vec![0; 0x80000].try_into().expect("spu ram alloc"),
        }
    }
}

impl Spu {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            enabled: self.control.enabled(),
            muted: !self.control.unmuted(),
            main_volume_left: i16_volume_to_percent(self.main_volume.l.volume()),
            main_volume_right: i16_volume_to_percent(self.main_volume.r.volume()),
            voices: std::array::from_fn(|i| {
                let v = &self.voices[i];
                VoiceSnapshot {
                    start_address: (v.start_address >> 3) as u16,
                    repeat_address: (v.repeat_address >> 3) as u16,
                    current_address: v.current_address >> 3,
                    sample_rate: sample_rate_to_hz(v.sample_rate),
                    volume_left: i16_volume_to_percent(v.volume.l.volume()),
                    volume_right: i16_volume_to_percent(v.volume.r.volume()),
                    adsr_phase: v.envelope.phase(),
                    adsr_volume: i16_volume_to_percent(v.envelope.volume()),
                }
            }),
        }
    }

    pub fn dma_read(&mut self) -> u32 {
        let addr = self.current_address;
        let bytes = [
            self.sound_ram[addr],
            self.sound_ram[addr + 1],
            self.sound_ram[addr + 2],
            self.sound_ram[addr + 3],
        ];

        self.current_address += 4;
        u32::from_le_bytes(bytes)
    }

    pub fn dma_write(&mut self, data: u32) {
        let bytes = data.to_le_bytes();
        let addr = self.current_address & 0x7FFFF;

        self.sound_ram[addr] = bytes[0];
        self.sound_ram[addr + 1] = bytes[1];
        self.sound_ram[addr + 2] = bytes[2];
        self.sound_ram[addr + 3] = bytes[3];

        self.current_address += 4;
    }

    pub fn tick(system: &mut System) -> Option<[i16; 2]> {
        let spu = &mut system.spu;

        if !spu.control.enabled() {
            return None;
        }

        let mut mixed_l = 0_i32;
        let mut mixed_r = 0_i32;

        for voice in &mut spu.voices {
            let samples = voice.tick(spu.sound_ram.as_slice());
            mixed_l += i32::from(samples[0]);
            mixed_r += i32::from(samples[1]);
        }

        let cd_l = system.cdrom.pop_from_audio_buffer().unwrap_or(0);
        let cd_r = system.cdrom.pop_from_audio_buffer().unwrap_or(0);

        if !spu.control.unmuted() {
            return Some([0, 0]);
        }

        mixed_l += i32::from(apply_volume(cd_l, spu.cd_volume.l));
        mixed_r += i32::from(apply_volume(cd_r, spu.cd_volume.r));

        let clamped_l = mixed_l.clamp(-32768, 32767) as i16;
        let clamped_r = mixed_r.clamp(-32768, 32767) as i16;

        let output_l = apply_volume(clamped_l, spu.main_volume.l.volume());
        let output_r = apply_volume(clamped_r, spu.main_volume.r.volume());

        Some([output_l, output_r])
    }
}

#[allow(clippy::match_same_arms)]
pub fn read<const WIDTH: usize>(system: &System, addr: u32) -> u32 {
    trace!("spu read addr={addr:08x}");

    let spu = &system.spu;

    match addr {
        0x1F80_1D80 => u32::from(spu.main_volume.l.register.0),
        0x1F80_1D82 => u32::from(spu.main_volume.r.register.0),

        0x1F80_1DB8 => spu.main_volume.l.volume() as u32, // current volume
        0x1F80_1DBA => spu.main_volume.r.volume() as u32,

        0x1F80_1DAA => spu.control.0.into(), // Status register
        0x1F80_1DAE => (spu.control.0 & 0x3F).into(),

        0x1F80_1D88 => spu.voice_key_on,
        0x1F80_1D8A => spu.voice_key_on >> 16,

        0x1F80_1D8C => spu.voice_key_off,
        0x1F80_1D8E => spu.voice_key_off >> 16,

        0x1F80_1DA6 => spu.data_transfer_address.into(),
        0x1F80_1DAC => spu.data_transfer_control.into(),

        0x1F80_1DB0 => spu.cd_volume.l as u32,
        0x1F80_1DB2 => spu.cd_volume.r as u32,

        0x1F80_1DB4 => spu.external_volume.l as u32,
        0x1F80_1DB6 => spu.external_volume.r as u32,

        0x1F80_1D94 => spu.voice_noise_enable,
        0x1F80_1D96 => spu.voice_noise_enable >> 16,

        0x1F80_1D98 => spu.voice_echo_on,
        0x1F80_1D9A => spu.voice_echo_on >> 16,

        0x1F80_1D90 => spu.voice_pitch_enable,
        0x1F80_1D92 => spu.voice_pitch_enable >> 16,

        0x1F80_1D9C => 0, // voice status (read only)
        0x1F80_1D9E => 0, // voice status (read only)

        0x1F80_1D84 => 0, // spu.reverb.output_volume.l = val as i16,
        0x1F80_1D86 => 0, // spu.reverb.output_volume.r = val as i16,
        0x1F80_1DA2 => 0, // spu.reverb.base = val,

        0x1F80_1DC0 => 0, // spu.reverb.apf_offset[0] = val,
        0x1F80_1DC2 => 0, // spu.reverb.apf_offset[1] = val,

        0x1F80_1DD0 => 0, // spu.reverb.apf_volume[0] = val,
        0x1F80_1DD2 => 0, // spu.reverb.apf_volume[1] = val,

        0x1F80_1DF4 => 0, // spu.reverb.apf_address[0].l = val as i16,
        0x1F80_1DF6 => 0, // spu.reverb.apf_address[0].r = val as i16,
        0x1F80_1DF8 => 0, // spu.reverb.apf_address[1].l = val as i16,
        0x1F80_1DFA => 0, // spu.reverb.apf_address[1].r = val as i16,

        0x1F80_1DC4 => 0, // spu.reverb.reflection_volume[0] = val,
        0x1F80_1DCE => 0, // spu.reverb.reflection_volume[1] = val,

        0x1F80_1DD4 => 0, // spu.reverb.same_side_reflect_addr[0].l = val as i16,
        0x1F80_1DD6 => 0, // spu.reverb.same_side_reflect_addr[0].r = val as i16,
        0x1F80_1DE0 => 0, // spu.reverb.same_side_reflect_addr[1].l = val as i16,
        0x1F80_1DE2 => 0, // spu.reverb.same_side_reflect_addr[1].r = val as i16,

        0x1F80_1DE4 => 0, // spu.reverb.diff_side_reflect_addr[0].l = val as i16,
        0x1F80_1DE6 => 0, // spu.reverb.diff_side_reflect_addr[0].r = val as i16,
        0x1F80_1DF0 => 0, // spu.reverb.diff_side_reflect_addr[1].l = val as i16,
        0x1F80_1DF2 => 0, // spu.reverb.diff_side_reflect_addr[1].r = val as i16,

        0x1F80_1DC6 => 0, // spu.reverb.comb_volume[0] = val,
        0x1F80_1DC8 => 0, // spu.reverb.comb_volume[1] = val,
        0x1F80_1DCA => 0, // spu.reverb.comb_volume[2] = val,
        0x1F80_1DCC => 0, // spu.reverb.comb_volume[3] = val,

        0x1F80_1DD8 => 0, // spu.reverb.comb_address[0].l = val as i16,
        0x1F80_1DDA => 0, // spu.reverb.comb_address[0].r = val as i16,
        0x1F80_1DDC => 0, // spu.reverb.comb_address[1].l = val as i16,
        0x1F80_1DDE => 0, // spu.reverb.comb_address[1].r = val as i16,
        0x1F80_1DE8 => 0, // spu.reverb.comb_address[2].l = val as i16,
        0x1F80_1DEA => 0, // spu.reverb.comb_address[2].r = val as i16,
        0x1F80_1DEC => 0, // spu.reverb.comb_address[3].l = val as i16,
        0x1F80_1DEE => 0, // spu.reverb.comb_address[3].r = val as i16,

        0x1F80_1DFC => 0, // spu.reverb.input_volume.l = val as i16,
        0x1F80_1DFE => 0, // spu.reverb.input_volume.r = val as i16,

        0x1F80_1DA4 => (spu.sound_irq_address >> 3) as u32,

        0x1F80_1C00..=0x1F80_1D7F => {
            let offset = addr - 0x1F80_1C00;
            let idx = (offset / 0x10) as usize;
            let reg = offset % 0x10;

            let voice = &spu.voices[idx];

            match reg {
                0x00 => u32::from(voice.volume.l.register.0),
                0x02 => u32::from(voice.volume.r.register.0),
                0x04 => u32::from(voice.sample_rate),

                0x06 => voice.start_address / 8,
                0x0E => voice.repeat_address / 8,

                0x08 => voice.envelope.register.0,
                0x0A => voice.envelope.register.0 >> 16,

                0x0C => voice.envelope.volume() as u32,

                x => unimplemented!("spu voice reg read {x}"),
            }
        }

        // Misc unknown registers
        0x1F80_1DA0 => 0x9D78,
        0x1F80_1DBC => 0x8021_4BDF,
        0x1F80_1E00..=0x1F80_1E7F => {
            warn!("spu reading from unknown register");
            0
        }

        x => unimplemented!("spu read {x:8X}, width={}", WIDTH * 8),
    }
}

#[allow(clippy::match_same_arms)]
pub fn write<const WIDTH: usize>(system: &mut System, addr: u32, val: u32) {
    // To make generics more readable
    const HIGH: bool = true;
    const LOW: bool = false;

    trace!("spu write addr={addr:08x}, data={:08x}", val);
    debug_assert_ne!(WIDTH, 1);

    if WIDTH == 4 {
        let lo = val & 0xFFFF;
        let hi = (val >> 16) & 0xFFFF;

        write::<2>(system, addr, lo);
        write::<2>(system, addr + 2, hi);

        return;
    }

    let spu = &mut system.spu;
    let val = val as u16;

    match addr {
        0x1F80_1D80 => spu.main_volume.l.set_volume(val),
        0x1F80_1D82 => spu.main_volume.r.set_volume(val),

        0x1F80_1DAE => {} // spu status read only

        0x1F80_1D84 => {} // spu.reverb.output_volume.l = val as i16,
        0x1F80_1D86 => {} // spu.reverb.output_volume.r = val as i16,
        0x1F80_1DA2 => {} // spu.reverb.base = val,

        0x1F80_1DC0 => {} // spu.reverb.apf_offset[0] = val,
        0x1F80_1DC2 => {} // spu.reverb.apf_offset[1] = val,

        0x1F80_1DD0 => {} // spu.reverb.apf_volume[0] = val,
        0x1F80_1DD2 => {} // spu.reverb.apf_volume[1] = val,

        0x1F80_1DF4 => {} // spu.reverb.apf_address[0].l = val as i16,
        0x1F80_1DF6 => {} // spu.reverb.apf_address[0].r = val as i16,
        0x1F80_1DF8 => {} // spu.reverb.apf_address[1].l = val as i16,
        0x1F80_1DFA => {} // spu.reverb.apf_address[1].r = val as i16,

        0x1F80_1DC4 => {} // spu.reverb.reflection_volume[0] = val,
        0x1F80_1DCE => {} // spu.reverb.reflection_volume[1] = val,

        0x1F80_1DD4 => {} // spu.reverb.same_side_reflect_addr[0].l = val as i16,
        0x1F80_1DD6 => {} // spu.reverb.same_side_reflect_addr[0].r = val as i16,
        0x1F80_1DE0 => {} // spu.reverb.same_side_reflect_addr[1].l = val as i16,
        0x1F80_1DE2 => {} // spu.reverb.same_side_reflect_addr[1].r = val as i16,

        0x1F80_1DE4 => {} // spu.reverb.diff_side_reflect_addr[0].l = val as i16,
        0x1F80_1DE6 => {} // spu.reverb.diff_side_reflect_addr[0].r = val as i16,
        0x1F80_1DF0 => {} // spu.reverb.diff_side_reflect_addr[1].l = val as i16,
        0x1F80_1DF2 => {} // spu.reverb.diff_side_reflect_addr[1].r = val as i16,

        0x1F80_1DC6 => {} // spu.reverb.comb_volume[0] = val,
        0x1F80_1DC8 => {} // spu.reverb.comb_volume[1] = val,
        0x1F80_1DCA => {} // spu.reverb.comb_volume[2] = val,
        0x1F80_1DCC => {} // spu.reverb.comb_volume[3] = val,

        0x1F80_1DD8 => {} // spu.reverb.comb_address[0].l = val as i16,
        0x1F80_1DDA => {} // spu.reverb.comb_address[0].r = val as i16,
        0x1F80_1DDC => {} // spu.reverb.comb_address[1].l = val as i16,
        0x1F80_1DDE => {} // spu.reverb.comb_address[1].r = val as i16,
        0x1F80_1DE8 => {} // spu.reverb.comb_address[2].l = val as i16,
        0x1F80_1DEA => {} // spu.reverb.comb_address[2].r = val as i16,
        0x1F80_1DEC => {} // spu.reverb.comb_address[3].l = val as i16,
        0x1F80_1DEE => {} // spu.reverb.comb_address[3].r = val as i16,

        0x1F80_1DFC => {} // spu.reverb.input_volume.l = val as i16,
        0x1F80_1DFE => {} // spu.reverb.input_volume.r = val as i16,

        0x1F80_1DAA => spu.control.0 = val,
        0x1F80_1DA4 => spu.sound_irq_address = usize::from(val) * 8,

        0x1F80_1D8C => {
            write_half::<LOW>(&mut spu.voice_key_off, val);
            for i in 0..16 {
                if spu.voice_key_off & (1 << i) != 0 {
                    spu.voices[i].key_off();
                }
            }
        }
        0x1F80_1D8E => {
            write_half::<HIGH>(&mut spu.voice_key_off, val);
            for i in 16..24 {
                if spu.voice_key_off & (1 << i) != 0 {
                    spu.voices[i].key_off();
                }
            }
        }
        0x1F80_1D88 => {
            write_half::<LOW>(&mut spu.voice_key_on, val);
            for i in 0..16 {
                if spu.voice_key_on & (1 << i) != 0 {
                    spu.voices[i].key_on(spu.sound_ram.as_slice());
                }
            }
        }
        0x1F80_1D8A => {
            write_half::<HIGH>(&mut spu.voice_key_on, val);
            for i in 16..24 {
                if spu.voice_key_on & (1 << i) != 0 {
                    spu.voices[i].key_on(spu.sound_ram.as_slice());
                }
            }
        }

        0x1F80_1D90 => write_half::<LOW>(&mut spu.voice_pitch_enable, val),
        0x1F80_1D92 => write_half::<HIGH>(&mut spu.voice_pitch_enable, val),

        0x1F80_1D94 => write_half::<LOW>(&mut spu.voice_noise_enable, val),
        0x1F80_1D96 => write_half::<HIGH>(&mut spu.voice_noise_enable, val),

        0x1F80_1D98 => write_half::<LOW>(&mut spu.voice_echo_on, val),
        0x1F80_1D9A => write_half::<HIGH>(&mut spu.voice_echo_on, val),

        0x1F80_1DB0 => spu.cd_volume.l = val as i16,
        0x1F80_1DB2 => spu.cd_volume.r = val as i16,

        0x1F80_1DB4 => spu.external_volume.l = val as i16,
        0x1F80_1DB6 => spu.external_volume.r = val as i16,

        0x1F80_1DAC => spu.data_transfer_control = val, // Should be 0x0004

        0x1F80_1DA6 => {
            spu.data_transfer_address = val;
            spu.current_address = usize::from(val) * 8;
        }

        // Transfer half word to ram
        0x1F80_1DA8 => {
            let bytes = val.to_le_bytes();
            let addr = spu.current_address;

            spu.sound_ram[addr] = bytes[0];
            spu.sound_ram[addr + 1] = bytes[1];
            spu.current_address += 2;
        }

        0x1F80_1C00..=0x1F80_1D7F => {
            let offset = addr - 0x1F80_1C00;
            let idx = (offset / 0x10) as usize;
            let reg = offset % 0x10;

            let voice = &mut spu.voices[idx];

            match reg {
                0x00 => voice.volume.l.set_volume(val),
                0x02 => voice.volume.r.set_volume(val),
                0x04 => voice.sample_rate = val,

                0x06 => voice.start_address = u32::from(val) * 8,
                0x0E => voice.set_repeat_address(val),

                0x08 => write_half::<LOW>(&mut voice.envelope.register.0, val),
                0x0A => write_half::<HIGH>(&mut voice.envelope.register.0, val),

                0x0C => voice.envelope.set_volume(val as i16),

                x => unimplemented!("spu voice reg write {x}"),
            }
        }

        0x1F80_1D9C => {} // voice status (read only)
        0x1F80_1D9E => {} // voice status (read only)

        0x1F80_1E00..=0x1F80_1E7F => {
            warn!("spu writing to unknown register");
        }

        x => unimplemented!("spu write {x:8X}"),
    }
}

#[derive(Default)]
struct SweepVolume {
    l: SweepVolumeLR,
    r: SweepVolumeLR,
}

#[derive(Default)]
struct Volume {
    l: i16,
    r: i16,
}

fn apply_volume(sample: i16, volume: i16) -> i16 {
    ((i32::from(sample) * i32::from(volume)) >> 15) as i16
}

fn i16_volume_to_percent(v: i16) -> f32 {
    (f32::from(v) / 0x7FFF as f32) * 100.0
}

fn sample_rate_to_hz(raw: u16) -> f32 {
    f32::from(raw) / 4096.0 * 44100.0
}

pub use envelope::AdsrPhase;

pub struct VoiceSnapshot {
    pub start_address: u16,
    pub repeat_address: u16,
    pub current_address: usize,
    pub sample_rate: f32,
    pub volume_left: f32,
    pub volume_right: f32,
    pub adsr_phase: AdsrPhase,
    pub adsr_volume: f32,
}

pub struct Snapshot {
    pub enabled: bool,
    pub muted: bool,
    pub main_volume_left: f32,
    pub main_volume_right: f32,
    pub voices: [VoiceSnapshot; 24],
}
