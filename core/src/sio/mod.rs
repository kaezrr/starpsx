use arrayvec::ArrayVec;

use crate::{
    System,
    mem::ByteAddressable,
    sched::{DevicePort, Event, SerialSend},
    sio::gamepad::Gamepad,
};

mod gamepad;

pub const PADDR_START: u32 = 0x1F801040;
pub const PADDR_END: u32 = 0x1F80105F;

bitfield::bitfield! {
    struct Status(u32);
    rx_ready, set_rx_ready : 1;
    dsr_input_on, set_dsr_input_on: 7;
    irq, set_irq : 9;
}

impl Default for Status {
    fn default() -> Self {
        Self(5) // TX idle and TX ready
    }
}

bitfield::bitfield! {
#[derive(Default)]
    struct Control(u16);
    tx_enabled, _ : 0;
    dtr_output_on, _ : 1;
    rx_enabled, _ : 2;
    acknowlegde, _ : 4;
    reset, _ : 6;
    dsr_interrupt_enable, _ : 12;
}

#[derive(Default)]
pub struct SerialInterface {
    received: ArrayVec<u8, 4>,
    status: Status,
    control: Control,

    // Not used, just there to make reads consistent
    mode: u32,
    baud_timer_reload_value: u16,

    gamepad: Gamepad,
}

impl SerialInterface {
    fn read(&mut self, offs: u32) -> u32 {
        eprintln!("serial read {offs:02x}");
        match offs {
            0x0 => self.pop_received_data(),
            0x4 => self.status.0,
            0x8 => self.mode,
            0xA => self.control.0.into(),
            0xE => self.baud_timer_reload_value.into(),
            _ => unimplemented!("serial read {offs:02x}"),
        }
    }

    fn write(system: &mut System, offs: u32, val: u32) {
        let sio = &mut system.sio;

        eprintln!("serial write {offs:02x} <- {val:08x}");
        match offs {
            0x0 => Self::send_data(system, val as u8),
            0x8 => sio.mode = val & 0x1FF,
            0xA => sio.write_control(val as u16),
            0xE => sio.baud_timer_reload_value = val as u16,
            _ => unimplemented!("serial write {offs:02x} <- {val:08x}"),
        }
    }

    fn write_control(&mut self, val: u16) {
        // Unused bits
        self.control.0 = val & !0xC000;

        if self.control.acknowlegde() {
            self.status.set_dsr_input_on(false);
        }

        if self.control.reset() {
            self.reset_registers();
        }

        if !self.control.dtr_output_on() {
            self.gamepad.reset();
        }

        self.status.set_dsr_input_on(self.control.dtr_output_on());
    }

    fn send_data(system: &mut System, val: u8) {
        println!("SEND {:02x}", val);
        system
            .scheduler
            .schedule(Event::Serial(SerialSend::new(0x01, val)), 1500, None);
    }

    fn pop_received_data(&mut self) -> u32 {
        let data = self.received.pop_at(0).unwrap_or_default();

        if self.received.is_empty() {
            self.status.set_rx_ready(false);
        }

        println!("RECEIVED {:02x}", data);
        data.into()
    }

    fn reset_registers(&mut self) {
        *self = Self::default();
    }

    pub fn push_received_data(&mut self, data: u8) {
        self.received.pop_at(0);
        self.received.push(data);
        self.status.set_rx_ready(true);
    }

    pub fn process_serial_send(system: &mut System, send: SerialSend) {
        match send.port {
            DevicePort::Gamepad => Gamepad::send_and_receive_byte(system, send.data),
            DevicePort::MemoryCard => todo!("Memory card not yet implemented"),
        }
    }
}

pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> T {
    let offs = addr - PADDR_START;

    let data = match offs & 0x10 {
        0 => system.sio.read(offs % 0x10),
        1 => unimplemented!("SIO1 read"),
        _ => unreachable!(),
    };

    T::from_u32(data)
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, val: T) {
    let offs = addr - PADDR_START;

    match offs & 0x10 {
        0 => SerialInterface::write(system, offs % 0x10, val.to_u32()),
        1 => unimplemented!("SIO1 write {val:08x}"),
        _ => unreachable!(),
    }
}
