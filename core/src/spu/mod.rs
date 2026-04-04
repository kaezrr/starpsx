mod snapshot;
mod voice;

pub use snapshot::AdsrPhase;
pub use snapshot::Snapshot;
pub use snapshot::VoiceSnapshot;
use tracing::debug;
use tracing::warn;

use crate::System;
use crate::spu::voice::Voice;

pub const PADDR_START: u32 = 0x1F80_1C00;
pub const PADDR_END: u32 = 0x1F80_1E80;

bitfield::bitfield! {
    #[derive(Default)]
    struct Control(u16);
    enabled, _ : 15; // Doesn't affect CD Audio
    unmuted, _ : 14; // Doesn't affect CD Audio
}

pub struct Spu {
    control: Control,

    main_volume: Volume<Sweep>,
    cd_volume: Volume<i16>,
    ex_volume: Volume<i16>,

    voice_pitch_enable: u32,
    voice_noise_enable: u32,
    voice_reverb_enable: u32,

    ram_data_transfer_control: u16,
    ram_data_transfer_address: u16,

    voice_key_off: u32,
    voice_key_on: u32,

    // Internal registers
    current_address: usize,
    voices: [Voice; 24],

    sound_ram: Box<[u8; 0x80000]>,
}

impl Default for Spu {
    fn default() -> Self {
        Self {
            control: Control::default(),

            main_volume: Volume::default(),
            cd_volume: Volume::default(),
            ex_volume: Volume::default(),

            voice_pitch_enable: 0,
            voice_noise_enable: 0,
            voice_reverb_enable: 0,

            ram_data_transfer_control: 0,
            ram_data_transfer_address: 0,

            voice_key_off: 0,
            voice_key_on: 0,

            current_address: 0,
            voices: Default::default(),

            sound_ram: vec![0; 0x80000].try_into().expect("alloc sound ram"),
        }
    }
}

impl Spu {
    pub fn ram_read<const WIDTH: usize>(&mut self) -> u32 {
        let addr = self.current_address & 0x7FFFF;
        let mut buffer = [0u8; 4];

        buffer[..WIDTH].copy_from_slice(&self.sound_ram[addr..addr + WIDTH]);
        self.current_address += WIDTH;

        u32::from_le_bytes(buffer)
    }

    pub fn ram_write<const WIDTH: usize>(&mut self, word: u32) {
        let addr = self.current_address & 0x7FFFF;
        let bytes = word.to_le_bytes();

        self.sound_ram[addr..addr + WIDTH].copy_from_slice(&bytes[..WIDTH]);
        self.current_address += WIDTH;
    }

    pub fn tick(system: &mut System) -> [i16; 2] {
        let spu = &mut system.spu;

        if !spu.control.enabled() {
            return [0, 0];
        }

        let mut mixed_l = 0i32;
        let mut mixed_r = 0i32;

        for v in &mut spu.voices {
            let samples = v.tick(spu.sound_ram.as_ref());

            if !v.keyed_on {
                continue;
            }

            mixed_l += i32::from(samples[0]);
            mixed_r += i32::from(samples[1]);
        }

        if !spu.control.unmuted() {
            return [0, 0];
        }

        let clamped_sample_l = mixed_l.clamp(-0x7FFF, 0x8000) as i16;
        let clamped_sample_r = mixed_r.clamp(-0x7FFF, 0x8000) as i16;

        let output_l = apply_volume(clamped_sample_l, spu.main_volume.l.0);
        let output_r = apply_volume(clamped_sample_r, spu.main_volume.r.0);

        [output_l, output_r]
    }

    const fn write_control(&mut self, value: u16) {
        self.control.0 = value;
    }

    /// Bits 0-5 of the control register mirror the status register
    /// Other bits not emulated
    const fn status(&self) -> u16 {
        self.control.0 & 0x3F
    }

    fn write_key_off<const HIGH: usize>(&mut self, val: u16) {
        write_half::<HIGH>(&mut self.voice_key_off, val);

        let base = HIGH * 16;
        let count = if HIGH == 1 { 8 } else { 16 };

        for i in 0..count {
            if (val >> i) & 1 != 0 {
                self.voices[base + i].key_off();
            }
        }
    }

    fn write_key_on<const HIGH: usize>(&mut self, val: u16) {
        write_half::<HIGH>(&mut self.voice_key_on, val);

        let base = HIGH * 16;
        let count = if HIGH == 1 { 8 } else { 16 };

        for i in 0..count {
            if (val >> i) & 1 != 0 {
                self.voices[base + i].key_on(self.sound_ram.as_ref());
            }
        }
    }

    fn write_pitch_enable<const HIGH: usize>(&mut self, val: u16) {
        write_half::<HIGH>(&mut self.voice_pitch_enable, val);

        let base = HIGH * 16;
        let count = if HIGH == 1 { 8 } else { 16 };

        for i in 0..count {
            self.voices[base + i].pulse_modulation_enabled = (val >> i) & 1 != 0;
        }
    }

    const fn write_noise_enable<const HIGH: usize>(&mut self, val: u16) {
        write_half::<HIGH>(&mut self.voice_noise_enable, val);

        // TODO: Loop through voices modify their noise flags
    }

    const fn write_reverb_enable<const HIGH: usize>(&mut self, val: u16) {
        write_half::<HIGH>(&mut self.voice_reverb_enable, val);

        // TODO: Loop through voices modify their noise flags
    }

    fn write_transfer_address(&mut self, val: u16) {
        self.ram_data_transfer_address = val;
        self.current_address = usize::from(val) * 8;
    }
}

/// 8bit, 16bit and 32bit reads are supported
pub fn read<const WIDTH: usize>(system: &System, addr: u32) -> u32 {
    let spu = &system.spu;

    debug!("spu read {addr:08x}");

    match addr {
        // 24 Voices
        0x1F80_1C00..=0x1F80_1D7F => {
            let i = ((addr - 0x1F80_1C00) / 0x10) as usize;
            let r = ((addr - 0x1F80_1C00) % 0x10) as usize;

            match r {
                0xC => {
                    if spu.voices[i].keyed_on {
                        0x7FFF
                    } else {
                        0
                    }
                }
                _ => unimplemented!("read voice {i} register {r:x}"),
            }
        }

        0x1F80_1D88 => spu.voice_key_on,
        0x1F80_1D8A => spu.voice_key_on >> 16,

        0x1F80_1D8C => spu.voice_key_off,
        0x1F80_1D8E => spu.voice_key_off >> 16,

        0x1F80_1DAA => u32::from(spu.control.0),
        0x1F80_1DAC => u32::from(spu.ram_data_transfer_control), // should be 0x0004
        0x1F80_1DAE => u32::from(spu.status()),

        0x1F80_1DA6 => u32::from(spu.ram_data_transfer_address),

        0x1F80_1DB8 => spu.main_volume.l.0 as u32, // TODO: current
        0x1F80_1DBA => spu.main_volume.r.0 as u32, // TODO: current

        x => unimplemented!("spu read {x:8X}, width={}", WIDTH * 8),
    }
}

///  16bit writes are suppored,
pub fn write<const WIDTH: usize>(system: &mut System, addr: u32, val: u32) {
    //  32bit writes are also supported but seem to be particularly unstable
    //  So they are split into 2 16bit writes instead
    if WIDTH == 4 {
        write::<2>(system, addr, val);
        write::<2>(system, addr + 2, val);
        return;
    }

    //  8bit writes to ODD addresses are simply ignored
    //  8bit writes to EVEN addresses are executed as 16bit writes
    if WIDTH == 1 {
        if addr & 1 == 0 {
            write::<2>(system, addr, val);
        }
        return;
    }

    let spu = &mut system.spu;
    let val = val as u16;

    debug!("spu write {addr:08x} <- {val:04x}");

    match addr {
        // 24 Voices
        0x1F80_1C00..=0x1F80_1D7F => {
            let i = ((addr - 0x1F80_1C00) / 0x10) as usize;
            let r = ((addr - 0x1F80_1C00) % 0x10) as usize;

            match r {
                0x0 => spu.voices[i].volume.set_l(val),
                0x2 => spu.voices[i].volume.set_r(val),
                0x4 => spu.voices[i].set_sample_rate(val),
                0x6 => spu.voices[i].set_start_address(val),
                0x8 => warn!("set voice {i} ADSR low"),
                0xA => warn!("set voice {i} ADSR high"),
                0xC => spu.voices[i].adsr_volume = val.cast_signed(),
                0xE => spu.voices[i].set_repeat_address(val),

                _ => unimplemented!("write voice {i} register {r:x} {val:x}"),
            }
        }

        0x1F80_1D80 => spu.main_volume.set_l(val),
        0x1F80_1D82 => spu.main_volume.set_r(val),

        0x1F80_1D88 => spu.write_key_on::<0>(val),
        0x1F80_1D8A => spu.write_key_on::<1>(val),

        0x1F80_1D8C => spu.write_key_off::<0>(val),
        0x1F80_1D8E => spu.write_key_off::<1>(val),

        0x1F80_1D90 => spu.write_pitch_enable::<0>(val),
        0x1F80_1D92 => spu.write_pitch_enable::<1>(val),

        0x1F80_1D94 => spu.write_noise_enable::<0>(val),
        0x1F80_1D96 => spu.write_noise_enable::<1>(val),

        0x1F80_1D98 => spu.write_reverb_enable::<0>(val),
        0x1F80_1D9A => spu.write_reverb_enable::<1>(val),

        0x1F80_1DAA => spu.write_control(val),

        0x1F80_1DA6 => spu.write_transfer_address(val),
        0x1F80_1DA8 => spu.ram_write::<2>(u32::from(val)),
        0x1F80_1DAC => spu.ram_data_transfer_control = val, // should be 0x0004

        0x1F80_1DB0 => spu.cd_volume.set_l(val),
        0x1F80_1DB2 => spu.cd_volume.set_r(val),

        0x1F80_1DB4 => spu.ex_volume.set_l(val),
        0x1F80_1DB6 => spu.ex_volume.set_r(val),

        // Reverb stuff is stubbed out for now
        0x1F80_1D84 | 0x1F80_1D86 => warn!("writing reverb volume"),
        0x1F80_1D9C | 0x1F80_1D9E => {} // ENDX Read only

        0x1F80_1DA2 => warn!("writing reverb work area start address"),
        0x1F80_1DC0..=0x1F80_1DFE => warn!("writing reverb configuration"),

        x => unimplemented!("spu write {x:8X} {val:x}"),
    }
}

pub fn signed4bit(v: u8) -> i32 {
    i32::from((v as i8) << 4 >> 4)
}

const fn write_half<const HIGH: usize>(reg: &mut u32, val: u16) {
    debug_assert!(HIGH <= 1, "invalid word half");

    let shift = HIGH * 16;
    let mask = 0xFFFF << shift;

    *reg = (*reg & !mask) | ((val as u32) << shift);
}

#[derive(Default)]
pub struct Volume<T> {
    l: T,
    r: T,
}

impl Volume<Sweep> {
    const fn set_l(&mut self, v: u16) {
        debug_assert!(v & 0x8000 == 0, "Fixed volume");
        self.l.0 = v.cast_signed() << 1;
    }

    const fn set_r(&mut self, v: u16) {
        debug_assert!(v & 0x8000 == 0, "Fixed volume");
        self.r.0 = v.cast_signed() << 1;
    }
}

impl Volume<i16> {
    const fn set_l(&mut self, v: u16) {
        self.l = v.cast_signed();
    }

    const fn set_r(&mut self, v: u16) {
        self.r = v.cast_signed();
    }
}

#[derive(Default)]
struct Sweep(i16);

fn apply_volume(sample: i16, volume: i16) -> i16 {
    ((i32::from(sample) * i32::from(volume)) >> 15) as i16
}
