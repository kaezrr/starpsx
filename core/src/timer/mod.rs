mod timer0;
mod timer1;
mod timer2;

use timer0::Timer0;
use timer1::Timer1;
use timer2::Timer2;

pub const PADDR_START: u32 = 0x1F801100;
pub const PADDR_END: u32 = 0x1F80112F;

bitfield::bitfield! {
    #[derive(Default)]
    pub struct TimerMode(u32);
    sync_enable, _ : 0;
    sync_mode, _ : 2, 1;
    reset_to_target, _ : 3;
    irq_target, _ : 4;
    irq_ffff, _ : 5;
    irq_repeat, _: 6;
    irq_toggle, _: 7;
    clock_src, _: 9, 8;
    interrupt, set_interrupt: 10;
    reached_target, set_reached_target: 11;
    reached_ffff, set_reached_ffff: 12;
}

#[derive(Default)]
pub struct Timers {
    timer0: Timer0,
    timer1: Timer1,
    timer2: Timer2,
}

impl Timers {
    pub fn read_reg(&mut self, addr: u32) -> u32 {
        let addr = addr - PADDR_START;
        let offs = addr % 0x10;
        let timer = (addr / 0x10) as usize;

        match timer {
            0 => self.timer0.read(offs),
            1 => self.timer1.read(offs),
            2 => self.timer2.read(offs),
            _ => panic!("invalid timer read {timer}"),
        }
    }

    pub fn write_reg(&mut self, addr: u32, val: u32) {
        let addr = addr - PADDR_START;
        let offs = addr % 0x10;
        let timer = (addr / 0x10) as usize;

        match timer {
            0 => self.timer0.write(offs, val),
            1 => self.timer1.write(offs, val),
            2 => self.timer2.write(offs, val),
            _ => panic!("invalid timer write {timer}"),
        }
    }
}
