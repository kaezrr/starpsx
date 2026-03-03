mod utils;

use tracing::debug;

use crate::System;
use crate::mem::ByteAddressable;

use utils::write_half;

pub const PADDR_START: u32 = 0x1F801C00;
pub const PADDR_END: u32 = 0x1F801E7F;

pub struct Spu {
    volume: Volume,
    cd_volume: Volume,
    external_volume: Volume,

    voice_key_off: u32,
    voice_key_on: u32,
    voice_pitch_enable: u32,
    voice_noise_enable: u32,
    voice_echo_on: u32,

    voices: [Voice; 24],
    reverb: Reverb,

    data_transfer_address: u16,
    current_address: usize,

    control: u16,

    sound_ram: Box<[u8; 0x80000]>,
}

impl Default for Spu {
    fn default() -> Self {
        Self {
            volume: Volume::default(),
            cd_volume: Volume::default(),
            external_volume: Volume::default(),

            voice_key_off: 0,
            voice_key_on: 0,
            voice_pitch_enable: 0,
            voice_noise_enable: 0,
            voice_echo_on: 0,

            voices: std::array::from_fn(|_| Voice::default()),

            reverb: Reverb::default(),

            data_transfer_address: 0,
            current_address: 0,

            control: 0,

            sound_ram: Box::new([0; 0x80000]),
        }
    }
}

pub fn read<T: ByteAddressable>(system: &System, addr: u32) -> T {
    debug!("spu read addr={addr:08x}");

    let spu = &system.spu;

    let data = match addr {
        0x1F801DB8 => spu.volume.left as u32,

        0x1F801DAE => 0, // TODO: status
        0x1F801DAA => spu.control as u32,

        0x1F801D88 => spu.voice_key_on,
        0x1F801D8A => spu.voice_key_on >> 16,

        0x1F801D8C => spu.voice_key_off,
        0x1F801D8E => spu.voice_key_off >> 16,

        0x1F801DA6 => spu.data_transfer_address as u32,
        0x1F801DAC => 0x0004, // RAM data tranfer control

        0x1F801C00..=0x1F801D7F => {
            let offset = addr - 0x1F801C00;
            let idx = (offset / 0x10) as usize;
            let reg = offset % 0x10;

            let voice = &spu.voices[idx];

            match reg {
                0x0C => voice.adsr_volume as u32,

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
    const LOW: bool = true;

    debug!("spu write addr={addr:08x}, data={:08x}", val.to_u32());
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
        0x1F801D80 => spu.volume.left = val,
        0x1F801D82 => spu.volume.right = val,

        0x1F801D84 => spu.reverb.output_volume.left = val,
        0x1F801D86 => spu.reverb.output_volume.right = val,
        0x1F801DA2 => spu.reverb.base = val,

        0x1F801DC0 => spu.reverb.apf_offset[0] = val,
        0x1F801DC2 => spu.reverb.apf_offset[1] = val,

        0x1F801DD0 => spu.reverb.apf_volume[0] = val,
        0x1F801DD2 => spu.reverb.apf_volume[1] = val,

        0x1F801DF4 => spu.reverb.apf_address[0].left = val,
        0x1F801DF6 => spu.reverb.apf_address[0].right = val,
        0x1F801DF8 => spu.reverb.apf_address[1].left = val,
        0x1F801DFA => spu.reverb.apf_address[1].right = val,

        0x1F801DC4 => spu.reverb.reflection_volume[0] = val,
        0x1F801DCE => spu.reverb.reflection_volume[1] = val,

        0x1F801DD4 => spu.reverb.same_side_reflect_addr[0].left = val,
        0x1F801DD6 => spu.reverb.same_side_reflect_addr[0].right = val,
        0x1F801DE0 => spu.reverb.same_side_reflect_addr[1].left = val,
        0x1F801DE2 => spu.reverb.same_side_reflect_addr[1].right = val,

        0x1F801DE4 => spu.reverb.diff_side_reflect_addr[0].left = val,
        0x1F801DE6 => spu.reverb.diff_side_reflect_addr[0].right = val,
        0x1F801DF0 => spu.reverb.diff_side_reflect_addr[1].left = val,
        0x1F801DF2 => spu.reverb.diff_side_reflect_addr[1].right = val,

        0x1F801DC6 => spu.reverb.comb_volume[0] = val,
        0x1F801DC8 => spu.reverb.comb_volume[1] = val,
        0x1F801DCA => spu.reverb.comb_volume[2] = val,
        0x1F801DCC => spu.reverb.comb_volume[3] = val,

        0x1F801DD8 => spu.reverb.comb_address[0].left = val,
        0x1F801DDA => spu.reverb.comb_address[0].right = val,
        0x1F801DDC => spu.reverb.comb_address[1].left = val,
        0x1F801DDE => spu.reverb.comb_address[1].right = val,
        0x1F801DE8 => spu.reverb.comb_address[2].left = val,
        0x1F801DEA => spu.reverb.comb_address[2].right = val,
        0x1F801DEC => spu.reverb.comb_address[3].left = val,
        0x1F801DEE => spu.reverb.comb_address[3].right = val,

        0x1F801DFC => spu.reverb.input_volume.left = val,
        0x1F801DFE => spu.reverb.input_volume.right = val,

        0x1F801DAA => spu.control = val,

        0x1F801D8C => {
            write_half::<LOW>(&mut spu.voice_key_off, val);
            // for i in 0..16 {
            //     if spu.voice_key_off & (1 << i) != 0 {
            //         spu.voices[i].key_off();
            //     }
            // }
        }
        0x1F801D8E => {
            write_half::<HIGH>(&mut spu.voice_key_off, val);
            // for i in 16..24 {
            //     if spu.voice_key_off & (1 << i) != 0 {
            //         spu.voices[i].key_off();
            //     }
            // }
        }
        0x1F801D88 => {
            write_half::<LOW>(&mut spu.voice_key_on, val);
            // for i in 0..16 {
            //     if spu.voice_key_on & (1 << i) != 0 {
            //         spu.voices[i].key_on();
            //     }
            // }
        }
        0x1F801D8A => {
            write_half::<HIGH>(&mut spu.voice_key_on, val);
            // for i in 16..24 {
            //     if spu.voice_key_on & (1 << i) != 0 {
            //         spu.voices[i].key_on();
            //     }
            // }
        }

        0x1F801D90 => write_half::<LOW>(&mut spu.voice_pitch_enable, val),
        0x1F801D92 => write_half::<HIGH>(&mut spu.voice_pitch_enable, val),

        0x1F801D94 => write_half::<LOW>(&mut spu.voice_noise_enable, val),
        0x1F801D96 => write_half::<HIGH>(&mut spu.voice_noise_enable, val),

        0x1F801D98 => write_half::<LOW>(&mut spu.voice_echo_on, val),
        0x1F801D9A => write_half::<HIGH>(&mut spu.voice_echo_on, val),

        0x1F801DB0 => spu.cd_volume.left = val,
        0x1F801DB2 => spu.cd_volume.right = val,

        0x1F801DB4 => spu.external_volume.left = val,
        0x1F801DB6 => spu.external_volume.right = val,

        0x1F801DAC => assert_eq!(val, 0x0004, "Sound RAM Data Control is {val:x} != 0x0004"),

        0x1F801DA6 => {
            spu.data_transfer_address = val;
            spu.current_address = spu.data_transfer_address as usize * 8;
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
                0x00 => voice.volume.left = val,
                0x02 => voice.volume.right = val,
                0x04 => voice.sample_rate = val,

                0x06 => voice.start_address = (val as u32) << 3,
                0x0E => voice.repeat_address = (val as u32) << 3,

                0x08 => write_half::<LOW>(&mut voice.adsr, val),
                0x0A => write_half::<HIGH>(&mut voice.adsr, val),

                0x0C => voice.adsr_volume = val,

                x => unimplemented!("spu voice reg write {x}"),
            }
        }

        0x1F801D9C => {} // voice status (read only)
        0x1F801D9E => {} // voice status (read only)

        x => unimplemented!("spu write {x:8X}"),
    }
}

#[derive(Clone, Copy, Default)]
struct Volume {
    left: u16,
    right: u16,
}

#[derive(Default)]
struct Voice {
    volume: Volume,
    sample_rate: u16,

    adsr: u32,
    adsr_volume: u16,

    start_address: u32,
    current_address: u32,
    repeat_address: u32,

    pitch_counter: u16,

    decode_buffer: [i16; 28],
    current_buffer_idx: u8,

    keyed_on: bool,

    current_sample: i16,
    previous_sample: i16,
}

#[derive(Default)]
struct Reverb {
    output_volume: Volume,
    base: u16,

    apf_offset: [u16; 2],
    apf_volume: [u16; 2],
    apf_address: [Volume; 2],

    reflection_volume: [u16; 2],
    same_side_reflect_addr: [Volume; 2],
    diff_side_reflect_addr: [Volume; 2],

    comb_volume: [u16; 4],
    comb_address: [Volume; 4],

    input_volume: Volume,
}
