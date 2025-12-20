mod cd_image;
mod commands;

use std::ops::Div;

use arrayvec::ArrayVec;
use tracing::trace;

use crate::{System, mem::ByteAddressable, sched::Event};

pub use cd_image::CdImage;
pub use commands::ResponseType;

pub const PADDR_START: u32 = 0x1F801800;
pub const PADDR_END: u32 = 0x1F801803;

bitfield::bitfield! {
    #[derive(Default)]
    struct Address(u8);
    bank, _: 1, 0;
    adpcm_busy, _ : 2;
    param_empty, set_param_empty : 3;
    param_write_ready, set_param_write_ready : 4;
    result_read_ready, set_result_read_ready : 5;
    data_request, _ : 6;
    busy, _ : 7;
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
    pub _, set_shell_open: 4;
    pub _, set_seeking: 6;
}

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

enum SectorSize {
    DataOnly,
    WholeSectorExceptSyncBytes,
}

pub struct CdRom {
    address: Address,
    hintsts: Hintsts,
    hintmsk: Hintmsk,
    parameters: ArrayVec<u8, 16>,
    results: Vec<u8>,

    speed: Speed,
    sector_size: SectorSize,

    disc: Option<CdImage>,

    pub status: Status,
}

impl Default for CdRom {
    fn default() -> Self {
        Self {
            // Parameters empty and ready to write
            address: Address(0x18),
            hintsts: Hintsts::default(),
            hintmsk: Hintmsk::default(),
            parameters: ArrayVec::default(),
            results: Vec::new(),
            disc: None,
            // Motor on, shell open
            status: Status(0x12),
            speed: Speed::Normal,
            sector_size: SectorSize::DataOnly,
        }
    }
}

impl CdRom {
    // Only bit 0-1 are writable
    fn write_addr(&mut self, val: u8) {
        trace!(
            bank = self.address.bank(),
            "cdrom write address={:#02x}", val
        );
        self.address.0 = (self.address.0 & !3) | (val & 3);
    }

    fn read_addr(&self) -> u8 {
        trace!(
            bank = self.address.bank(),
            "cdrom read address={:#02x}", self.address.0
        );
        self.address.0
    }

    // Clear the corresponding set bit of HINTSTS
    fn write_hclrctl(&mut self, val: u8) {
        trace!(
            bank = self.address.bank(),
            "cdrom write hclrctl={:#02x}", val
        );
        self.hintsts.0 &= !(val & 0x1F)
    }

    fn write_hintmsk(&mut self, val: u8) {
        trace!(
            bank = self.address.bank(),
            "cdrom write hintmsk={:#02x}", val
        );
        self.hintmsk.0 = val;
    }

    fn read_hintsts(&self) -> u8 {
        trace!(
            bank = self.address.bank(),
            "cdrom read hintsts={:#02x}", self.hintsts.0
        );
        self.hintsts.0 | 0xE0 // Bits 5-7 are always 1 on read
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
        let responses = match cmd {
            0x01 => cdrom.nop(),
            0x19 => cdrom.test(),
            0x1A => cdrom.get_id(),
            0x02 => cdrom.set_loc(),
            0x15 => cdrom.seekl(),
            0x0E => cdrom.setmode(),
            _ => unimplemented!("cdrom command {cmd:02x}"),
        };

        responses.get().into_iter().for_each(|(res_type, delay)| {
            system
                .scheduler
                .schedule(Event::CdromResultIrq(res_type), delay, None)
        });
    }

    pub fn handle_response(system: &mut System, response: ResponseType) {
        let cdrom = &mut system.cdrom;

        cdrom.results.clear();
        match response {
            ResponseType::INT3(response) => {
                cdrom.results.extend(response);
                cdrom.hintsts.set_interrupt(3);
            }
            ResponseType::INT2(response) => {
                cdrom.results.extend(response);
                cdrom.hintsts.set_interrupt(2);
            }
            ResponseType::INT2Seek => {
                cdrom.status.set_seeking(false);
                cdrom.results.extend(vec![cdrom.status.0]);
                cdrom.hintsts.set_interrupt(2);
            }
        }

        cdrom.parameters.clear();
        cdrom.address.set_param_empty(true);
        cdrom.address.set_param_write_ready(true);
        cdrom.address.set_result_read_ready(true);

        if cdrom.hintsts.0 & cdrom.hintmsk.0 != 0 {
            system.irqctl.stat().set_cdrom(true);
        }
    }

    pub fn insert_disc(&mut self, image: CdImage) {
        self.disc = Some(image);

        // Reset cdrom state
        self.parameters.clear();
        self.results.clear();
    }
}

pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> T {
    let offs = addr - PADDR_START;
    let cdrom = &mut system.cdrom;

    let val: u8 = match (cdrom.address.bank(), offs) {
        (_, 0) => cdrom.read_addr(),
        (1, 1) => cdrom.pop_result(),
        (1, 3) => cdrom.read_hintsts(),
        (x, y) => unimplemented!("cdrom read bank {x} reg {y}"),
    };

    T::from_u32(u32::from(val))
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, data: T) {
    let offs = addr - PADDR_START;
    let cdrom = &mut system.cdrom;
    let val = data.to_u8();

    match (cdrom.address.bank(), offs) {
        (_, 0) => cdrom.write_addr(val),
        (0, 1) => CdRom::exec_command(system, val),
        (0, 2) => cdrom.push_parameter(val),
        (1, 3) => cdrom.write_hclrctl(val),
        (1, 2) => cdrom.write_hintmsk(val),
        (x, y) => unimplemented!("cdrom write bank {x} reg {y} <- {data:08x}"),
    }
}
