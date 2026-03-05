mod envelope;
mod utils;
mod voice;

use num_enum::{FromPrimitive, IntoPrimitive};
use tracing::trace;

use crate::mem::ByteAddressable;
use crate::{System, spu::envelope::SweepVolume};

use utils::{GAUSSIAN_TABLE, write_half};
use voice::Voice;

pub const PADDR_START: u32 = 0x1F801C00;
pub const PADDR_END: u32 = 0x1F801E7F;

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
    main_volume: Volume,

    cd_volume: u32,
    external_volume: u32,

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

    sound_ram: Box<[u8; 0x80000]>,
}

impl Default for Spu {
    fn default() -> Self {
        Self {
            main_volume: Volume::default(),

            cd_volume: 0,
            external_volume: 0,

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

            sound_ram: Box::new([0; 0x80000]),
        }
    }
}

impl Spu {
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
        let addr = self.current_address;

        self.sound_ram[addr] = bytes[0];
        self.sound_ram[addr + 1] = bytes[1];
        self.sound_ram[addr + 2] = bytes[2];
        self.sound_ram[addr + 3] = bytes[3];

        self.current_address += 4;
    }

    pub fn tick(&mut self) -> (i16, i16) {
        if !self.control.enabled() {
            return (0, 0);
        }

        let mut mixed_l = 0_i32;
        let mut mixed_r = 0_i32;

        for voice in self.voices.iter_mut() {
            let samples = voice.tick(self.sound_ram.as_slice());
            mixed_l += i32::from(samples.0);
            mixed_r += i32::from(samples.1);
        }

        if !self.control.unmuted() {
            return (0, 0);
        }

        // Clamp the sums to 16-bit
        let clamped_l = mixed_l.clamp(-0x8000, 0x7FFF) as i16;
        let clamped_r = mixed_r.clamp(-0x8000, 0x7FFF) as i16;

        // Apply main volume
        let output_l = apply_volume(clamped_l, self.main_volume.l.volume());
        let output_r = apply_volume(clamped_r, self.main_volume.r.volume());

        (output_l, output_r)
    }
}

pub fn read<T: ByteAddressable>(system: &System, addr: u32) -> T {
    trace!("spu read addr={addr:08x}");

    let spu = &system.spu;

    let data = match addr {
        0x1F801DB8 => spu.main_volume.l.volume() as u32,
        0x1F801DBA => spu.main_volume.r.volume() as u32,

        0x1F801DAE => (spu.control.0 & 0x3F).into(),

        0x1F801DAA => spu.control.0.into(), // Status register

        0x1F801D88 => spu.voice_key_on,
        0x1F801D8A => spu.voice_key_on >> 16,

        0x1F801D8C => spu.voice_key_off,
        0x1F801D8E => spu.voice_key_off >> 16,

        0x1F801DA6 => spu.data_transfer_address.into(),
        0x1F801DAC => spu.data_transfer_control.into(),

        0x1F801D98 => spu.voice_echo_on,
        0x1F801D9A => spu.voice_echo_on >> 16,

        0x1F801C00..=0x1F801D7F => {
            let offset = addr - 0x1F801C00;
            let idx = (offset / 0x10) as usize;
            let reg = offset % 0x10;

            let voice = &spu.voices[idx];

            match reg {
                0x0C => voice.envelope.volume() as u32,

                0x08 => voice.envelope.register.0,
                0x0A => voice.envelope.register.0 >> 16,

                x => unimplemented!("spu voice reg read {x}"),
            }
        }

        x => unimplemented!("spu read {x:8X}, width={}", T::LEN * 8),
    };

    T::from_u32(data)
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, val: T) {
    // To make generics more readable
    const HIGH: bool = true;
    const LOW: bool = false;

    trace!("spu write addr={addr:08x}, data={:08x}", val.to_u32());
    debug_assert_ne!(T::LEN, 1);

    if T::LEN == 4 {
        let val = val.to_u32();
        let lo = val & 0xFFFF;
        let hi = (val >> 16) & 0xFFFF;

        write::<u16>(system, addr, lo as u16);
        write::<u16>(system, addr + 2, hi as u16);

        return;
    }

    let spu = &mut system.spu;
    let val = val.to_u16();

    match addr {
        0x1F801D80 => spu.main_volume.l.set_volume(val),
        0x1F801D82 => spu.main_volume.r.set_volume(val),

        0x1F801D84 => {} // spu.reverb.output_volume.l = val as i16,
        0x1F801D86 => {} // spu.reverb.output_volume.r = val as i16,
        0x1F801DA2 => {} // spu.reverb.base = val,

        0x1F801DC0 => {} // spu.reverb.apf_offset[0] = val,
        0x1F801DC2 => {} // spu.reverb.apf_offset[1] = val,

        0x1F801DD0 => {} // spu.reverb.apf_volume[0] = val,
        0x1F801DD2 => {} // spu.reverb.apf_volume[1] = val,

        0x1F801DF4 => {} // spu.reverb.apf_address[0].l = val as i16,
        0x1F801DF6 => {} // spu.reverb.apf_address[0].r = val as i16,
        0x1F801DF8 => {} // spu.reverb.apf_address[1].l = val as i16,
        0x1F801DFA => {} // spu.reverb.apf_address[1].r = val as i16,

        0x1F801DC4 => {} // spu.reverb.reflection_volume[0] = val,
        0x1F801DCE => {} // spu.reverb.reflection_volume[1] = val,

        0x1F801DD4 => {} // spu.reverb.same_side_reflect_addr[0].l = val as i16,
        0x1F801DD6 => {} // spu.reverb.same_side_reflect_addr[0].r = val as i16,
        0x1F801DE0 => {} // spu.reverb.same_side_reflect_addr[1].l = val as i16,
        0x1F801DE2 => {} // spu.reverb.same_side_reflect_addr[1].r = val as i16,

        0x1F801DE4 => {} // spu.reverb.diff_side_reflect_addr[0].l = val as i16,
        0x1F801DE6 => {} // spu.reverb.diff_side_reflect_addr[0].r = val as i16,
        0x1F801DF0 => {} // spu.reverb.diff_side_reflect_addr[1].l = val as i16,
        0x1F801DF2 => {} // spu.reverb.diff_side_reflect_addr[1].r = val as i16,

        0x1F801DC6 => {} // spu.reverb.comb_volume[0] = val,
        0x1F801DC8 => {} // spu.reverb.comb_volume[1] = val,
        0x1F801DCA => {} // spu.reverb.comb_volume[2] = val,
        0x1F801DCC => {} // spu.reverb.comb_volume[3] = val,

        0x1F801DD8 => {} // spu.reverb.comb_address[0].l = val as i16,
        0x1F801DDA => {} // spu.reverb.comb_address[0].r = val as i16,
        0x1F801DDC => {} // spu.reverb.comb_address[1].l = val as i16,
        0x1F801DDE => {} // spu.reverb.comb_address[1].r = val as i16,
        0x1F801DE8 => {} // spu.reverb.comb_address[2].l = val as i16,
        0x1F801DEA => {} // spu.reverb.comb_address[2].r = val as i16,
        0x1F801DEC => {} // spu.reverb.comb_address[3].l = val as i16,
        0x1F801DEE => {} // spu.reverb.comb_address[3].r = val as i16,

        0x1F801DFC => {} // spu.reverb.input_volume.l = val as i16,
        0x1F801DFE => {} // spu.reverb.input_volume.r = val as i16,

        0x1F801DAA => spu.control.0 = val,

        0x1F801D8C => {
            write_half::<LOW>(&mut spu.voice_key_off, val);
            for i in 0..16 {
                if spu.voice_key_off & (1 << i) != 0 {
                    spu.voices[i].key_off();
                }
            }
        }
        0x1F801D8E => {
            write_half::<HIGH>(&mut spu.voice_key_off, val);
            for i in 16..24 {
                if spu.voice_key_off & (1 << i) != 0 {
                    spu.voices[i].key_off();
                }
            }
        }
        0x1F801D88 => {
            write_half::<LOW>(&mut spu.voice_key_on, val);
            for i in 0..16 {
                if spu.voice_key_on & (1 << i) != 0 {
                    spu.voices[i].key_on(spu.sound_ram.as_slice());
                }
            }
        }
        0x1F801D8A => {
            write_half::<HIGH>(&mut spu.voice_key_on, val);
            for i in 16..24 {
                if spu.voice_key_on & (1 << i) != 0 {
                    spu.voices[i].key_on(spu.sound_ram.as_slice());
                }
            }
        }

        0x1F801D90 => write_half::<LOW>(&mut spu.voice_pitch_enable, val),
        0x1F801D92 => write_half::<HIGH>(&mut spu.voice_pitch_enable, val),

        0x1F801D94 => write_half::<LOW>(&mut spu.voice_noise_enable, val),
        0x1F801D96 => write_half::<HIGH>(&mut spu.voice_noise_enable, val),

        0x1F801D98 => write_half::<LOW>(&mut spu.voice_echo_on, val),
        0x1F801D9A => write_half::<HIGH>(&mut spu.voice_echo_on, val),

        0x1F801DB0 => write_half::<LOW>(&mut spu.cd_volume, val),
        0x1F801DB2 => write_half::<HIGH>(&mut spu.cd_volume, val),

        0x1F801DB4 => write_half::<LOW>(&mut spu.external_volume, val),
        0x1F801DB6 => write_half::<HIGH>(&mut spu.external_volume, val),

        0x1F801DAC => spu.data_transfer_control = val, // Should be 0x0004

        0x1F801DA6 => {
            spu.data_transfer_address = val;
            spu.current_address = (spu.data_transfer_address as usize) << 3;
        }

        // Transfer half word to ram
        0x1F801DA8 => {
            let bytes = val.to_le_bytes();
            let addr = spu.current_address;

            spu.sound_ram[addr] = bytes[0];
            spu.sound_ram[addr + 1] = bytes[1];
            spu.current_address += 2;
        }

        0x1F801C00..=0x1F801D7F => {
            let offset = addr - 0x1F801C00;
            let idx = (offset / 0x10) as usize;
            let reg = offset % 0x10;

            let voice = &mut spu.voices[idx];

            match reg {
                0x00 => voice.volume.l.set_volume(val),
                0x02 => voice.volume.r.set_volume(val),
                0x04 => voice.sample_rate = val,

                0x06 => voice.start_address = val,
                0x0E => voice.repeat_address = val,

                0x08 => write_half::<LOW>(&mut voice.envelope.register.0, val),
                0x0A => write_half::<HIGH>(&mut voice.envelope.register.0, val),

                0x0C => voice.envelope.set_volume(val as i16),

                x => unimplemented!("spu voice reg write {x}"),
            }
        }

        0x1F801D9C => {} // voice status (read only)
        0x1F801D9E => {} // voice status (read only)

        x => unimplemented!("spu write {x:8X}"),
    }
}

#[derive(Default)]
struct Volume {
    l: SweepVolume,
    r: SweepVolume,
}

fn apply_volume(sample: i16, volume: i16) -> i16 {
    ((i32::from(sample) * i32::from(volume)) >> 15) as i16
}

// #[derive(Default)]
// struct Reverb {
//     output_volume: Volume,
//     base: u16,
//
//     apf_offset: [u16; 2],
//     apf_volume: [u16; 2],
//     apf_address: [Volume; 2],
//
//     reflection_volume: [u16; 2],
//     same_side_reflect_addr: [Volume; 2],
//     diff_side_reflect_addr: [Volume; 2],
//
//     comb_volume: [u16; 4],
//     comb_address: [Volume; 4],
//
//     input_volume: Volume,
// }
