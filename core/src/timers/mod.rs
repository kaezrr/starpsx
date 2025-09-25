mod timer0;
mod timer1;
mod timer2;

pub use timer0::Timer0;
pub use timer1::Timer1;
pub use timer2::Timer2;

use crate::{System, mem::ByteAddressable};

pub const PADDR_START: u32 = 0x1F801100;
pub const PADDR_END: u32 = 0x1F80112F;

#[derive(Clone, Copy, PartialEq)]
pub enum IRQMode {
    Pulse,
    Toggle,
}

impl From<u8> for IRQMode {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Toggle,
            0 => Self::Pulse,
            _ => unreachable!(),
        }
    }
}

bitfield::bitfield! {
    #[derive(Default)]
    pub struct TimerMode(u32);
    sync_enable, set_sync_enable : 0;
    sync_mode, _ : 2, 1;
    reset_to_target, _ : 3;
    irq_target, _ : 5, 4;
    irq_repeat, _: 6;
    u8, into IRQMode, irq_toggle, _: 7, 7;
    clock_src, _: 9, 8;
    pub irq_disable, set_irq_disable: 10;
    reached_target, set_reached_target: 11;
    reached_ffff, set_reached_ffff: 12;
}

#[derive(Default)]
pub struct Timers {
    pub timer0: Timer0,
    pub timer1: Timer1,
    pub timer2: Timer2,
}

impl Timers {
    pub fn read_reg(system: &mut System, addr: u32) -> u32 {
        let addr = addr - PADDR_START;
        let offs = addr % 0x10;
        let timer = (addr / 0x10) as usize;

        match timer {
            0 => Timer0::read(system, offs),
            1 => Timer1::read(system, offs),
            2 => Timer2::read(system, offs),
            _ => panic!("invalid timer read {timer}"),
        }
    }

    pub fn write_reg(system: &mut System, addr: u32, val: u32) {
        let addr = addr - PADDR_START;
        let offs = addr % 0x10;
        let timer = (addr / 0x10) as usize;

        match timer {
            0 => Timer0::write(system, offs, val),
            1 => Timer1::write(system, offs, val),
            2 => Timer2::write(system, offs, val),
            _ => panic!("invalid timer write {timer}"),
        }
    }

    pub fn enter_vblank(system: &mut System) {
        Timer1::enter_vblank(system);
    }

    pub fn exit_vblank(system: &mut System) {
        Timer1::exit_vblank(system);
    }

    pub fn enter_hblank(system: &mut System) {
        Timer0::enter_hblank(system);
        system.timers.timer1.increment_hblanks();
    }

    pub fn exit_hblank(system: &mut System) {
        Timer0::exit_hblank(system);
    }
}

pub fn read<T: ByteAddressable>(system: &mut System, offs: u32) -> T {
    T::from_u32(Timers::read_reg(system, offs))
}

pub fn write<T: ByteAddressable>(system: &mut System, offs: u32, data: T) {
    Timers::write_reg(system, offs, data.to_u32())
}
