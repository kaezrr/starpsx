use crate::{System, mem::ByteAddressable};

pub const PADDR_START: u32 = 0x1F801040;
pub const PADDR_END: u32 = 0x1F80105F;

bitfield::bitfield! {
    struct Status(u32);
    tx_ready, _ : 0;
    rx_ready, _ : 1;
    tx_idle, _ : 2;
    rx_parity_error, set_rx_parity_error : 3;
    dsr_input_on, set_dsr_input_on: 7;
    irq_request, set_irq_request : 9;
    baudrate_timer, _: 31, 11;
}

impl Default for Status {
    fn default() -> Self {
        Self(5)
    }
}

bitfield::bitfield! {
#[derive(Default)]
    struct Mode(u32);
    from into ReloadFactor, reload_factor, _ : 1, 0;
    from into CharLength, char_length, _ : 3, 2;
    parity_enabled, _ : 4;
    parity_odd, _ : 5;
    clock_pol_low, _ : 8;
}

bitfield::bitfield! {
#[derive(Default)]
    struct Control(u16);
    tx_enabled, _ : 0;
    dtr_output_on, _ : 1;
    rx_enabled, _ : 2;
    acknowlegde, _ : 4;
    reset, _ : 6;
}

#[derive(Default)]
struct SerialInterface {
    received_fifo: u32,
    transfer_fifo: u8,
    baud_timer_reload_value: u16,
    status: Status,
    mode: Mode,
    control: Control,
}

impl SerialInterface {
    fn read(&mut self, offs: u32) -> u32 {
        eprintln!("serial read {offs:02x}");
        match offs {
            0x0 => self.pop_recieved_data(),
            0x4 => self.status.0,
            // 0x8 => self.mode.0,
            0xA => self.control.0.into(),
            // 0xE => self.baud_timer_reload_value.into(),
            _ => unimplemented!("serial read {offs:02x}"),
        }
    }

    fn write(&mut self, offs: u32, val: u32) {
        eprintln!("serial write {offs:02x} <- {val:08x}");
        match offs {
            0x0 => self.send_data(val as u8),
            0x8 => self.write_mode(val),
            0xA => self.write_control(val as u16),
            0xE => self.baud_timer_reload_value = val as u16,
            _ => unimplemented!("serial write {offs:02x} <- {val:08x}"),
        }
    }

    fn write_mode(&mut self, val: u32) {
        // Unused bits
        self.mode.0 = val & 0x1F;
    }

    fn write_control(&mut self, val: u16) {
        // Unused bits
        self.control.0 = val & !0xC000;

        if self.control.acknowlegde() {
            self.status.set_rx_parity_error(false);
            self.status.set_irq_request(false);
            self.status.set_dsr_input_on(false);
        }

        if self.control.reset() {
            self.reset_registers();
        }
    }

    fn send_data(&mut self, val: u8) {
        if !self.status.tx_idle() || !self.control.tx_enabled() {
            panic!("Transfer not possible")
        }

        self.transfer_fifo = val;
    }

    fn pop_recieved_data(&mut self) -> u32 {
        let data = self.received_fifo;
        self.received_fifo >>= 8;
        data
    }

    fn reset_registers(&mut self) {
        self.received_fifo = 0;
        self.baud_timer_reload_value = 0;
        self.status = Status::default();
        self.mode.0 = 0;
        self.control.0 = 0;
    }
}

#[derive(Default)]
pub struct SerialInterfaces {
    sio0: SerialInterface,
}

pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> T {
    let offs = addr - PADDR_START;

    let data = match offs & 0x10 {
        0 => system.sio.sio0.read(offs % 0x10),
        1 => unimplemented!("SIO1 read"),
        _ => unreachable!(),
    };

    T::from_u32(data)
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, val: T) {
    let offs = addr - PADDR_START;

    match offs & 0x10 {
        0 => system.sio.sio0.write(offs % 0x10, val.to_u32()),
        1 => unimplemented!("SIO1 write {val:08x}"),
        _ => unreachable!(),
    }
}

enum ReloadFactor {
    MUL1,
    MUL16,
    MUL64,
}

enum CharLength {
    Bits5,
    Bits6,
    Bits7,
    Bits8,
}

impl From<u32> for ReloadFactor {
    fn from(v: u32) -> Self {
        match v {
            0 | 1 => Self::MUL1,
            2 => Self::MUL16,
            3 => Self::MUL64,
            _ => unreachable!(),
        }
    }
}

impl From<u32> for CharLength {
    fn from(v: u32) -> Self {
        match v {
            0 => Self::Bits5,
            1 => Self::Bits6,
            2 => Self::Bits7,
            3 => Self::Bits8,
            _ => unreachable!(),
        }
    }
}
