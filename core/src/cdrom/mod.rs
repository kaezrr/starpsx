use arrayvec::ArrayVec;

use crate::{System, mem::ByteAddressable};

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

pub struct CdRom {
    address: Address,
    hintsts: Hintsts,
    hintmsk: Hintmsk,
    parameters: ArrayVec<u8, 16>,
    results: ArrayVec<u8, 16>,
}

impl Default for CdRom {
    fn default() -> Self {
        let address = Address(0x18);
        let hintsts = Hintsts::default();
        let hintmsk = Hintmsk::default();
        let parameters = ArrayVec::default();
        let results = ArrayVec::default();
        Self {
            address,
            hintsts,
            hintmsk,
            parameters,
            results,
        }
    }
}

impl CdRom {
    // Only bit 0-1 are writable
    fn write_addr(&mut self, val: u8) {
        self.address.0 = (self.address.0 & !3) | (val & 3);
    }

    fn read_addr(&self) -> u8 {
        self.address.0
    }

    fn write_hclrctl(&mut self, val: u8) {
        let _register = Hclrctl(val);
    }

    fn write_hintmsk(&mut self, val: u8) {
        self.hintmsk.0 = val;
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

    fn test(&mut self, cmd: u8) -> Response {
        match cmd {
            // CDROM Version
            0x20 => Response::INT3([0x94, 0x09, 0x19, 0xC0].into()),
            _ => unimplemented!("cdrom command Test {cmd:02x}"),
        }
    }
}

fn exec_command(system: &mut System, cmd: u8) {
    let cdrom = &mut system.cdrom;
    let response = match cmd {
        0x19 => cdrom.test(cdrom.parameters[0]),
        _ => unimplemented!("cdrom command {cmd:02x}"),
    };

    match response {
        Response::INT3(array_vec) => {
            cdrom.results.clear();
            cdrom.results.extend(array_vec);
            cdrom.hintsts.set_interrupt(3);
        }
    }

    cdrom.parameters.clear();
    cdrom.address.set_param_empty(true);
    cdrom.address.set_param_write_ready(true);
    cdrom.address.set_result_read_ready(true);
    system.irqctl.stat().set_cdrom(true);
}

pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> T {
    let offs = addr - PADDR_START;
    let cdrom = &mut system.cdrom;

    let val: u8 = match (cdrom.address.bank(), offs) {
        (_, 0) => cdrom.read_addr(),
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
        (0, 1) => exec_command(system, val),
        (0, 2) => cdrom.push_parameter(val),
        (1, 3) => cdrom.write_hclrctl(val),
        (1, 2) => cdrom.write_hintmsk(val),
        (x, y) => unimplemented!("cdrom write bank {x} reg {y} <- {data:08x}"),
    }
}

enum Response {
    INT3(ArrayVec<u8, 4>),
}
