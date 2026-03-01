mod cd_image;
mod commands;

use std::collections::VecDeque;
use std::ops::Div;

use arrayvec::ArrayVec;
pub use cd_image::CdImage;
pub use commands::ResponseType;
use tracing::trace;

use crate::System;
use crate::consts::AVG_RATE_INT1;
use crate::mem::ByteAddressable;
use crate::sched::Event;

pub const PADDR_START: u32 = 0x1F801800;
pub const PADDR_END: u32 = 0x1F801803;

pub struct CdRom {
    status: Status,
    address: Address,
    hintsts: Hintsts,
    hintmsk: Hintmsk,
    parameters: ArrayVec<u8, 16>,
    results: Vec<u8>,

    speed: Speed,
    sector_size: SectorSize,
    sector_buffer: VecDeque<u8>,

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

            speed: Speed::Normal,
            sector_buffer: VecDeque::new(),
            sector_size: SectorSize::DataOnly,

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

    fn read_rddata<T: ByteAddressable>(&mut self) -> u32 {
        let mut bytes = [0u8; 4];

        (0..T::LEN).for_each(|i| {
            bytes[i] = self.pop_from_sector_buffer();
        });

        u32::from_le_bytes(bytes)
    }

    // Clear the corresponding set bit of HINTSTS
    fn write_hclrctl(&mut self, val: u8) {
        trace!(target:"cdrom", "cdrom write hclrctl={:#02x}", val);
        self.hintsts.0 &= !(val & 0x1F)
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

    fn write_hchpctl(&mut self, data: u8) {
        trace!(target:"cdrom", "cdrom write hchpctl={:#02x}", data);
    }

    fn replace_sector_buffer(&mut self, new_buffer: VecDeque<u8>) {
        self.sector_buffer = new_buffer;
        self.address.set_data_request(true);
    }

    fn pop_from_sector_buffer(&mut self) -> u8 {
        let data = self.sector_buffer.pop_front().unwrap();

        if self.sector_buffer.is_empty() {
            self.address.set_data_request(false);
        }

        data
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
                    ResponseType::INT1 => Some(cdrom.speed.transform(AVG_RATE_INT1)),
                    _ => None,
                };
                system
                    .scheduler
                    .schedule(Event::CdromResultIrq(res_type), delay, repeat)
            });
    }

    pub fn handle_response(system: &mut System, response: ResponseType) {
        let cdrom = &mut system.cdrom;

        cdrom.results.clear();
        let irq = match response {
            ResponseType::INT3(response) => {
                cdrom.results.extend(response);
                3
            }

            ResponseType::INT2(response) => {
                cdrom.results.extend(response);
                2
            }

            ResponseType::INT1 => {
                let sector_data = cdrom
                    .disk
                    .as_mut()
                    .unwrap()
                    .read_sector_and_advance(cdrom.sector_size);

                cdrom.replace_sector_buffer(sector_data);
                cdrom.results.extend(vec![cdrom.status.0]);
                1
            }

            ResponseType::INT5([status, error_code]) => {
                cdrom.results.push(status);
                cdrom.results.push(error_code);
                5
            }
        };

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

pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> T {
    let offs = addr - PADDR_START;
    let cdrom = &mut system.cdrom;

    let val: u32 = match (cdrom.address.bank(), offs) {
        (_, 0) => cdrom.read_addr().into(),
        (_, 1) => cdrom.pop_result().into(),
        (_, 2) => cdrom.read_rddata::<T>().to_u32(),
        (0, 3) | (2, 3) => cdrom.read_hintmsk().into(),
        (1, 3) | (3, 3) => cdrom.read_hintsts().into(),
        (x, y) => unreachable!("cdrom bank {x} register {y}"),
    };

    T::from_u32(val)
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, data: T) {
    let offs = addr - PADDR_START;
    let cdrom = &mut system.cdrom;
    let val = data.to_u8();

    match (cdrom.address.bank(), offs) {
        (_, 0) => cdrom.write_addr(val),

        (0, 1) => CdRom::exec_command(system, val),
        (0, 2) => cdrom.push_parameter(val),
        (0, 3) => cdrom.write_hchpctl(val),

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
    _, set_shell_open: 4;
    _, set_reading: 5;
    _, set_motor_on: 1;
    _, set_error: 0;
}

impl Status {
    /// Returns the status byte with the error bit set, without mutating self.
    pub fn with_error(&self) -> u8 {
        self.0 | 0x01
    }

    /// Clears the reading flag and returns the status byte before the change.
    pub fn clear_reading(&mut self) -> u8 {
        let before = self.0;
        self.set_reading(false);
        before
    }

    /// Sets the motor_on flag and returns the status byte before the change.
    pub fn enable_motor(&mut self) -> u8 {
        let before = self.0;
        self.set_motor_on(true);
        before
    }

    /// Clears the motor_on flag and returns the status byte before the change.
    pub fn disable_motor(&mut self) -> u8 {
        let before = self.0;
        self.set_motor_on(false);
        before
    }
}

#[derive(Clone, Copy, Debug)]
enum Speed {
    Normal,
    Double,
}

impl Speed {
    fn transform<T>(&self, value: T) -> T
    where
        T: Div<u64, Output = T>,
    {
        match self {
            Speed::Normal => value,
            Speed::Double => value / 2,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SectorSize {
    DataOnly,
    WholeSectorExceptSyncBytes,
}
