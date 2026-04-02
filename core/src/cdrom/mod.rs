mod cd_image;
mod cdxa_audio;
mod commands;

use std::collections::VecDeque;
use std::ops::Div;

use arrayvec::ArrayVec;
pub use cd_image::CdImage;
pub use commands::ResponseType;
use procmac::Boolable;
use tracing::trace;
use tracing::warn;

use crate::System;
use crate::cdrom::cdxa_audio::AdpcmHistory;
use crate::cdrom::cdxa_audio::BitsPerSample;
use crate::cdrom::cdxa_audio::Channel;
use crate::cdrom::cdxa_audio::HighResResampler;
use crate::cdrom::cdxa_audio::LowResResampler;
use crate::cdrom::cdxa_audio::SampleRate;
use crate::cdrom::cdxa_audio::decode_audio_sector;
use crate::consts::AVG_RATE_INT1;
use crate::sched::Event;

pub const PADDR_START: u32 = 0x1F80_1800;
pub const PADDR_END: u32 = 0x1F80_1804;

pub struct CdRom {
    status: Status,
    address: Address,
    hintsts: Hintsts,
    hintmsk: Hintmsk,
    parameters: ArrayVec<u8, 16>,
    results: Vec<u8>,

    mode: Mode,
    data_buffer: VecDeque<u8>,
    audio_buffer: VecDeque<i16>,
    audio_muted: bool,

    /// Left, Right, Mono
    adpcm_history: [AdpcmHistory; 3],
    high_res_resamplers: [HighResResampler; 3],
    low_res_resamplers: [LowResResampler; 3],

    filter_file: u8,
    filter_channel: u8,

    disk: Option<CdImage>,
}

impl Default for CdRom {
    fn default() -> Self {
        Self {
            // Parameters empty and ready to write
            status: Status(0),
            address: Address(0x18),
            hintsts: Hintsts::default(),
            hintmsk: Hintmsk::default(),
            parameters: ArrayVec::default(),
            results: Vec::new(),

            mode: Mode::default(),
            data_buffer: VecDeque::new(),
            audio_buffer: VecDeque::new(),
            audio_muted: false,

            adpcm_history: Default::default(),
            high_res_resamplers: Default::default(),
            low_res_resamplers: Default::default(),

            filter_file: 0,
            filter_channel: 0,

            disk: None,
        }
    }
}

impl CdRom {
    // Only bit 0-1 are writable
    fn write_addr(&mut self, val: u8) {
        trace!(target:"cdrom", "cdrom write address={:#02x}", val);
        self.address.0 = (self.address.0 & !3) | (val & 3);
    }

    fn read_addr(&self) -> u8 {
        trace!(target:"cdrom", "cdrom read address={:#02x}", self.address.0);
        self.address.0
    }

    pub fn read_rddata<const WIDTH: usize>(&mut self) -> u32 {
        let mut bytes = [0u8; 4];

        (0..WIDTH).for_each(|i| {
            bytes[i] = self.pop_from_data_buffer();
        });

        u32::from_le_bytes(bytes)
    }

    // Clear the corresponding set bit of HINTSTS
    fn write_hclrctl(&mut self, val: u8) {
        trace!(target:"cdrom", "cdrom write hclrctl={:#02x}", val);
        self.hintsts.0 &= !(val & 0x1F);
    }

    fn read_hintsts(&self) -> u8 {
        trace!(target:"cdrom", "cdrom read hintsts={:#02x}", self.hintsts.0);
        self.hintsts.0 | 0xE0 // Bits 5-7 are always 1 on read
    }

    fn write_hintmsk(&mut self, val: u8) {
        trace!(target:"cdrom", "cdrom write hintmsk={:#02x}", val);
        self.hintmsk.0 = val;
    }

    fn read_hintmsk(&self) -> u8 {
        trace!(target:"cdrom", "cdrom read hintmsk={:#02x}", self.hintmsk.0);
        self.hintmsk.0 | 0xE0 // Bits 5-7 are always 1 on read
    }

    fn push_to_data_buffer(&mut self, new_buffer: VecDeque<u8>) {
        self.data_buffer = new_buffer;
        self.address.set_data_request(true);
    }

    fn push_to_audio_buffer(&mut self, data: &[u8]) {
        let samples: Vec<i16> = data
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();

        self.audio_buffer.extend(samples);
    }

    fn pop_from_data_buffer(&mut self) -> u8 {
        let data = self.data_buffer.pop_front().unwrap_or_else(|| {
            warn!("cdrom pop from empty buffer");
            0
        });

        if self.data_buffer.is_empty() {
            self.address.set_data_request(false);
        }

        data
    }

    pub fn pop_from_audio_buffer(&mut self) -> Option<i16> {
        // If cdrom is muted then return 0 sample
        self.audio_buffer
            .pop_front()
            .map(|x| if self.audio_muted { 0 } else { x })
    }

    fn push_parameter(&mut self, val: u8) {
        if self.parameters.is_empty() {
            self.address.set_param_empty(false);
        }

        self.parameters.push(val);

        if self.parameters.is_full() {
            self.address.set_param_write_ready(false);
        }
    }

    fn pop_result(&mut self) -> u8 {
        let val = self.results.remove(0);
        if self.results.is_empty() {
            self.address.set_result_read_ready(false);
        }
        val
    }

    /// Returns whether the sector was as adpcm or data
    fn process_sector(&mut self, sector: Vec<u8>) -> bool {
        if self.status.playing() {
            assert!(self.mode.cdda_enabled); // CDDA bit should be on during play mode
            self.push_to_audio_buffer(&sector);
            return true;
        }

        let sector_mode = sector[0xF];
        let file = sector[0x10];
        let channel = sector[0x11];
        let submode = sector[0x12];
        let is_realtime_audio = (submode & 0x44) == 0x44;
        let is_form2 = submode & (1 << 5) != 0;
        let mode = &self.mode; // cdrom mode

        if sector_mode == 2 {
            // Filter rejects both ADPCM and data delivery if file/channel doesn't match
            if mode.filter_enabled && (file != self.filter_file || channel != self.filter_channel) {
                return false;
            }

            // ADPCM delivery
            if mode.adpcm_enabled && is_realtime_audio {
                let audio_header = cdxa_audio::AudioHeader(sector[0x13]);

                debug_assert!(is_form2);
                debug_assert_eq!(audio_header.bits_per_channel(), BitsPerSample::Bit4);
                debug_assert_ne!(audio_header.channel(), Channel::Reserved);

                let audio_samples = match (audio_header.channel(), audio_header.sample_rate()) {
                    (Channel::Mono, SampleRate::R37800) => decode_audio_sector::<false>(
                        &sector,
                        &mut self.adpcm_history,
                        &mut self.high_res_resamplers,
                    ),
                    (Channel::Stereo, SampleRate::R37800) => decode_audio_sector::<true>(
                        &sector,
                        &mut self.adpcm_history,
                        &mut self.high_res_resamplers,
                    ),
                    (Channel::Mono, SampleRate::R18900) => decode_audio_sector::<false>(
                        &sector,
                        &mut self.adpcm_history,
                        &mut self.low_res_resamplers,
                    ),
                    (Channel::Stereo, SampleRate::R18900) => decode_audio_sector::<true>(
                        &sector,
                        &mut self.adpcm_history,
                        &mut self.low_res_resamplers,
                    ),

                    (Channel::Reserved, _) => unimplemented!("Reserved cdxa audio num channels"),
                    (_, SampleRate::Reserved) => unimplemented!("Reserved cdxa sample rate"),
                };

                self.audio_buffer.extend(audio_samples);
                return true;
            }

            // Even if ADPCM is disabled, don't deliver realtime audio sectors as data
            // when filter is enabled
            if mode.filter_enabled && is_realtime_audio {
                return false;
            }
        }

        let mut sector_data = VecDeque::from(sector);
        match mode.sector_size {
            // Data is in the slice 0x18..0x818
            SectorSize::DataOnly => {
                sector_data.drain(0x818..);
                sector_data.drain(..0x18);
            }
            // Data is in the slice 0xC..
            SectorSize::WholeSectorExceptSyncBytes => {
                sector_data.drain(..0xC);
            }
        }

        self.push_to_data_buffer(sector_data);
        false
    }

    fn exec_command(system: &mut System, cmd: u8) {
        let cdrom = &mut system.cdrom;

        // Certain commands stop read responses
        if let 0x08..=0x09 = cmd {
            system
                .scheduler
                .unschedule(&Event::CdromResultIrq(ResponseType::INT1));
        }

        let response = match cmd {
            0x01 => cdrom.nop(),
            0x02 => cdrom.set_loc(),
            0x03 => cdrom.play(),
            0x06 => cdrom.readn(),
            0x08 => cdrom.stop(),
            0x09 => cdrom.pause(),
            0x0A => cdrom.init(),
            0x0C => cdrom.demute(),
            0x0D => cdrom.set_filter(),
            0x0E => cdrom.setmode(),
            0x11 => cdrom.get_locp(),
            0x13 => cdrom.get_tn(),
            0x14 => cdrom.get_td(),
            0x15 => cdrom.seekl(),
            0x16 => cdrom.seekp(),
            0x19 => cdrom.test(),
            0x1A => cdrom.get_id(),
            0x1B => cdrom.reads(),
            _ => unimplemented!("cdrom command {cmd:02x}"),
        };

        cdrom.parameters.clear();
        cdrom.address.set_param_empty(true);
        cdrom.address.set_param_write_ready(true);

        response
            .responses
            .into_iter()
            .for_each(|(res_type, delay)| {
                let repeat = match res_type {
                    ResponseType::INT1 => Some(cdrom.mode.speed.transform(AVG_RATE_INT1)),
                    _ => None,
                };
                system
                    .scheduler
                    .schedule(Event::CdromResultIrq(res_type), delay, repeat);
            });
    }

    pub fn handle_response(system: &mut System, response: ResponseType) {
        let cdrom = &mut system.cdrom;

        let irq = u8::from(&response);
        let mut results = Vec::new();

        match response {
            ResponseType::INT5(response) => {
                results.extend(response);
            }

            ResponseType::INT3(response) | ResponseType::INT2(response) => {
                results.extend(response);
            }

            ResponseType::INT1 => {
                let Some(inserted_disk) = cdrom.disk.as_mut() else {
                    panic!("int1 but no inserted disk");
                };

                let sector = inserted_disk.advance_sector();
                let sector_was_audio = cdrom.process_sector(sector);

                // Should not trigger any interrupts
                if sector_was_audio {
                    return;
                }

                results.push(cdrom.status.0);
            }
        }

        cdrom.results = results;
        cdrom.hintsts.set_interrupt(irq);
        cdrom.address.set_result_read_ready(true);

        if cdrom.hintsts.0 & cdrom.hintmsk.0 != 0 {
            system.irqctl.stat().set_cdrom(true);
        }
    }

    pub fn insert_disc(&mut self, image: CdImage) {
        self.disk = Some(image);

        // Reset cdrom state
        self.parameters.clear();
        self.results.clear();
    }

    pub fn open_shell(&mut self) {
        self.status.set_shell_open(true);
    }
}

pub fn read<const WIDTH: usize>(system: &mut System, addr: u32) -> u32 {
    let offs = addr - PADDR_START;
    let cdrom = &mut system.cdrom;

    match (cdrom.address.bank(), offs) {
        (_, 0) => cdrom.read_addr().into(),
        (_, 1) => cdrom.pop_result().into(),
        (_, 2) => cdrom.read_rddata::<WIDTH>(),
        (0 | 2, 3) => cdrom.read_hintmsk().into(),
        (1 | 3, 3) => cdrom.read_hintsts().into(),
        (x, y) => unreachable!("cdrom bank {x} register {y}"),
    }
}

pub fn write<const WIDTH: usize>(system: &mut System, addr: u32, data: u32) {
    let offs = addr - PADDR_START;
    let cdrom = &mut system.cdrom;
    let val = data as u8;

    match (cdrom.address.bank(), offs) {
        (_, 0) => cdrom.write_addr(val),

        (0, 1) => CdRom::exec_command(system, val),
        (0, 2) => cdrom.push_parameter(val),
        (0, 3) => trace!(target:"cdrom", "cdrom write hchpctl={data:#02x}"),

        (1, 2) => cdrom.write_hintmsk(val),
        (1, 3) => cdrom.write_hclrctl(val),

        (2, 2) => trace!(target:"cdrom", reg = "atv0", "cdrom ignored write to audio reg"),
        (2, 3) => trace!(target:"cdrom", reg = "atv1", "cdrom ignored write to audio reg"),

        (3, 1) => trace!(target:"cdrom", reg = "atv2", "cdrom ignored write to audio reg"),
        (3, 2) => trace!(target:"cdrom", reg = "atv3", "cdrom ignored write to audio reg"),
        (3, 3) => trace!(target:"cdrom", reg = "adpctl", "cdrom ignored write to audio reg"),

        (x, y) => unimplemented!("cdrom write bank {x} reg {y} <- {data:08x}"),
    }
}

bitfield::bitfield! {
    #[derive(Default)]
    struct Address(u8);
    bank, _: 1, 0;
    _, set_param_empty : 3;
    _, set_param_write_ready : 4;
    _, set_result_read_ready : 5;
    _, set_data_request: 6;
}

bitfield::bitfield! {
    #[derive(Default)]
    struct Hclrctl(u8);
    ack_hc05, _ : 2, 0;
    ack_bfempt, _ : 3;
    ack_bfbfwrdy, _ : 4;
    clear_sound_map, _: 5;
    clear_params, _ : 6;
    reset_decoder, _ : 7;
}

bitfield::bitfield! {
    #[derive(Default)]
    struct Hintsts(u8);
    interrupt, set_interrupt : 2, 0;
    sound_map_empty, _ : 3;
    sound_map_ready, _ : 4;
}

bitfield::bitfield! {
    #[derive(Default)]
    struct Hintmsk(u8);
    irq_intsts, _ : 2, 0;
    irq_bfempt, _ : 3;
    irq_bfwrdy, _ : 4;
}

bitfield::bitfield! {
    #[derive(Default)]
    pub struct Status(u8);
    playing, _: 7;
    _, set_shell_open: 4;
    _, set_motor_on: 1;
    _, set_error: 0;
}

// Reading/Seeking/Playing bits are mutually exclusive
impl Status {
    const STATE_MASK: u8 = (1 << 7) | (1 << 6) | (1 << 5);

    pub const fn set_reading(&mut self, value: bool) -> u8 {
        let before = self.0;
        if value {
            self.0 = (self.0 & !Self::STATE_MASK) | (1 << 5);
        } else {
            self.0 &= !(1 << 5);
        }
        before
    }

    pub const fn set_seeking(&mut self, value: bool) -> u8 {
        let before = self.0;
        if value {
            self.0 = (self.0 & !Self::STATE_MASK) | (1 << 6);
        } else {
            self.0 &= !(1 << 6);
        }
        before
    }

    pub const fn set_playing(&mut self, value: bool) -> u8 {
        let before = self.0;
        if value {
            self.0 = (self.0 & !Self::STATE_MASK) | (1 << 7);
        } else {
            self.0 &= !(1 << 7);
        }
        before
    }

    /// Returns the status byte with the error bit set, without mutating self.
    pub const fn with_error(&self) -> u8 {
        self.0 | 0x01
    }

    /// Sets the `motor_on` flag and returns the status byte before the change.
    pub fn enable_motor(&mut self) -> u8 {
        let before = self.0;
        self.set_motor_on(true);
        before
    }

    /// Clears the `motor_on` flag and returns the status byte before the change.
    pub fn disable_motor(&mut self) -> u8 {
        let before = self.0;
        self.set_motor_on(false);
        before
    }
}

#[derive(Clone, Copy, Debug, Boolable)]
enum Speed {
    Normal = 0,
    Double = 1,
}

impl Speed {
    fn transform<T>(self, value: T) -> T
    where
        T: Div<u64, Output = T>,
    {
        match self {
            Self::Normal => value,
            Self::Double => value / 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Boolable)]
pub enum SectorSize {
    DataOnly = 0,
    WholeSectorExceptSyncBytes = 1,
}

#[derive(Debug)]
struct Mode {
    speed: Speed,
    sector_size: SectorSize,
    adpcm_enabled: bool,
    filter_enabled: bool,
    cdda_enabled: bool,
    auto_pause: bool,
}

impl Mode {
    fn set_value(&mut self, data: u8) {
        self.speed = Speed::from(data & (1 << 7) != 0);
        self.adpcm_enabled = data & (1 << 6) != 0;
        self.filter_enabled = data & (1 << 3) != 0;
        self.cdda_enabled = data & 1 != 0;
        self.auto_pause = data & 2 != 0;

        // Set sector size only if ignore bit is 0
        if data & (1 << 4) == 0 {
            self.sector_size = SectorSize::from(data & (1 << 5) != 0);
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self {
            speed: Speed::Normal,
            sector_size: SectorSize::DataOnly,
            adpcm_enabled: false,
            filter_enabled: false,
            cdda_enabled: false,
            auto_pause: false,
        }
    }
}
