pub mod device_manager;
pub mod gamepad;

use arrayvec::ArrayVec;

use crate::{
    System, consts,
    mem::ByteAddressable,
    sched::Event,
    sio::{device_manager::DeviceManager, gamepad::Gamepad},
};

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
        Self(0x22005) // TX idle and TX ready
    }
}

bitfield::bitfield! {
#[derive(Default)]
    struct Control(u16);
    tx_enabled, _ : 0;
    dtr_output_on, _ : 1;
    rx_enabled, set_rx_enabled : 2;
    ack, set_ack : 4;
    reset, _ : 6;
    dsr_interrupt_enable, _ : 12;
    port_select, _: 13;
}

trait SerialInterface {
    fn read(&mut self, offs: u32) -> u32;
    fn write(system: &mut System, offs: u32, val: u32);
}

pub struct Sio0 {
    transfer: Option<u8>,
    received: ArrayVec<u8, 8>,
    status: Status,
    control: Control,

    // Not used, just there to make reads consistent
    mode: u32,
    baud_timer_reload_value: u16,

    device_manager: DeviceManager,
}

pub struct Sio1;

impl SerialInterface for Sio1 {
    fn read(&mut self, _offs: u32) -> u32 {
        0xFF
    }

    fn write(_system: &mut System, _offs: u32, _val: u32) {}
}

impl SerialInterface for Sio0 {
    fn read(&mut self, offs: u32) -> u32 {
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
        let sio = &mut system.sio0;
        match offs {
            0x0 => {
                sio.transfer = Some(val as u8);
                Self::try_send_data(system);
            }
            0x8 => sio.mode = val & 0x1FF,
            0xA => Self::write_control(system, val as u16),
            0xE => sio.baud_timer_reload_value = val as u16,
            _ => unimplemented!("serial write {offs:02x} <- {val:08x}"),
        }
    }
}

impl Sio0 {
    pub fn new(gamepads: [Option<Gamepad>; 2]) -> Self {
        Self {
            transfer: None,
            received: ArrayVec::default(),
            status: Status::default(),
            control: Control::default(),

            mode: 0,
            baud_timer_reload_value: 0,

            device_manager: DeviceManager::new(gamepads),
        }
    }

    pub fn turn_off_dsr(&mut self) {
        self.status.set_dsr_input_on(false);
    }

    fn write_control(system: &mut System, val: u16) {
        let sio = &mut system.sio0;
        // Unused bits
        sio.control.0 = val & !0xC000;

        // Resets STAT.3 4 5 9
        if sio.control.ack() {
            sio.status.set_irq(false);
            sio.control.set_ack(false);
        }

        if sio.control.reset() {
            sio.reset_registers();
        }

        if !sio.control.dtr_output_on() {
            sio.device_manager.reset();
            sio.status.set_dsr_input_on(false);
        }

        if sio.control.tx_enabled() {
            Self::try_send_data(system);
        }
    }

    fn try_send_data(system: &mut System) {
        // Can't transfer right now
        if !system.sio0.control.tx_enabled() {
            return;
        }

        // Transfer if there's something in TX
        if let Some(val) = system.sio0.transfer.take() {
            let (received, ack) = DeviceManager::send_and_receive_byte(system, val);

            let sio = &mut system.sio0;
            sio.status.set_dsr_input_on(ack);

            // 1088 cycles is the fixed baudrate delay for all games
            if sio.control.dsr_interrupt_enable() && sio.status.dsr_input_on() {
                system
                    .scheduler
                    .schedule(Event::SerialSend, consts::BAUDRATE_TRANSFER_DELAY, None);
            }

            // Turn off ACK after 10 cycles
            if sio.status.dsr_input_on() {
                system.scheduler.schedule(Event::DsrOff, 10, None);
            }

            system.sio0.push_received_data(received);
        }
    }

    fn pop_received_data(&mut self) -> u32 {
        let data = self.received.pop_at(0).unwrap_or(0xFF);

        if self.received.is_empty() {
            self.status.set_rx_ready(false);
        }

        data.into()
    }

    fn reset_registers(&mut self) {
        self.transfer = None;
        self.received = ArrayVec::default();
        self.status = Status::default();
        self.control = Control::default();

        self.mode = 0;
        self.baud_timer_reload_value = 0;

        self.device_manager.reset();
    }

    pub fn push_received_data(&mut self, data: u8) {
        // Ignore received data
        if !self.control.rx_enabled() && !self.control.dtr_output_on() {
            return;
        }

        if self.received.is_full() {
            *self.received.last_mut().unwrap() = data;
        } else {
            self.received.push(data);
        }

        self.status.set_rx_ready(true);
        self.control.set_rx_enabled(false);
    }

    pub fn process_serial_send(system: &mut System) {
        // Controller and Memory Card received byte interrupt
        system.irqctl.stat().set_ctl_mem(true);
        system.sio0.status.set_irq(true);
    }

    pub fn gamepad_port_0_mut(&mut self) -> &mut Gamepad {
        self.device_manager.gamepad_port_0_mut()
    }
}

pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> T {
    let offs = addr - PADDR_START;

    let data = match (offs & 0x10) >> 4 {
        0 => system.sio0.read(offs % 0x10),
        1 => system.sio1.read(offs % 0x10),
        _ => unreachable!(),
    };

    T::from_u32(data)
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, val: T) {
    let offs = addr - PADDR_START;

    match (offs & 0x10) >> 4 {
        0 => Sio0::write(system, offs % 0x10, val.to_u32()),
        1 => Sio1::write(system, offs % 0x10, val.to_u32()),
        _ => unreachable!(),
    }
}
