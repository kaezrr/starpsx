use std::ops::{Index, IndexMut};

use crate::{
    System,
    mem::ByteAddressable,
    sched::{Event, TimerInterrupt},
};

pub const PADDR_START: u32 = 0x1F801100;
pub const PADDR_END: u32 = 0x1F80112F;

#[derive(Default)]
pub struct Timers {
    timers: [Timer; 3],
    in_vsync: bool,
    in_hsync: bool,
    hblanks: u32,
}

impl Timers {
    fn clock_source(&self, which: usize) -> Clock {
        let source_raw = self[which].mode.clock_src();
        CLOCK_SOURCE_MATRIX[which][source_raw as usize]
    }

    fn sync_mode(&self, which: usize) -> SyncMode {
        let timer = &self[which];
        if timer.mode.sync_enabled() {
            let sync_raw = timer.mode.sync_mode();
            SYNC_MODE_MATRIX[which][sync_raw as usize]
        } else {
            SyncMode::FreeRun
        }
    }

    pub fn enter_vsync(system: &mut System) {
        system.timers.in_vsync = true;

        match system.timers.sync_mode(1) {
            SyncMode::ResetOnVSync | SyncMode::VSyncOnly => system.timers[1].counter = 0,
            _ => (),
        };
    }

    pub fn exit_vsync(system: &mut System) {
        system.timers.in_vsync = false;

        if system.timers.sync_mode(1) == SyncMode::StartOnNextFrame {
            system.timers[1].mode.set_sync_enabled(false);
        }
    }

    pub fn enter_hsync(system: &mut System) {
        system.timers.hblanks += 1;
        system.timers.in_hsync = true;

        match system.timers.sync_mode(0) {
            SyncMode::ResetOnHSync | SyncMode::HSyncOnly => system.timers[0].counter = 0,
            _ => (),
        };
    }

    pub fn exit_hsync(system: &mut System) {
        system.timers.in_hsync = false;

        if system.timers.sync_mode(0) == SyncMode::StartOnNextLine {
            system.timers[0].mode.set_sync_enabled(false);
        }
    }

    fn update_value(system: &mut System, which: usize) {
        let timer = &mut system.timers[which];

        let clock_delta = (system.scheduler.sysclk() - timer.last_read) as u32;
        timer.last_read = system.scheduler.sysclk();

        let hblanks_since_last_read = system.timers.hblanks;
        system.timers.hblanks = 0;

        match system.timers.sync_mode(which) {
            SyncMode::Paused => return,
            SyncMode::PauseOnHsync if system.timers.in_hsync => return,
            SyncMode::PauseOnVsync if system.timers.in_vsync => return,
            SyncMode::HSyncOnly if !system.timers.in_hsync => return,
            SyncMode::VSyncOnly if !system.timers.in_vsync => return,
            _ => (),
        }

        let timer = &mut system.timers[which];
        let reset = match timer.mode.reset_to_target() {
            true => timer.target as u32,
            false => 0xFFFF,
        };

        let delta = match system.timers.clock_source(which) {
            Clock::Cpu => clock_delta,
            Clock::CpuDiv8 => clock_delta / 8,
            Clock::Dot => clock_delta / system.gpu.get_dot_clock_divider(),
            Clock::HBlank => hblanks_since_last_read,
        };

        let timer = &mut system.timers[which];
        let ticks_until_target = timer.get_ticks_to_value(timer.target);
        let ticks_until_ffff = timer.get_ticks_to_value(0xFFFF);

        // Actual timer update
        timer.counter = (timer.counter + delta) % (reset + 1);

        // Set Reach Target and FFFF bits
        timer.mode.set_reached_target(delta >= ticks_until_target);
        timer.mode.set_reached_ffff(delta >= ticks_until_ffff);
    }

    pub fn process_interrupt(system: &mut System, irq: TimerInterrupt) {
        let timer = &mut system.timers[irq.which];
        let set_irq = if irq.toggle {
            let prev = timer.mode.irq_disabled();
            let next = !prev;
            timer.mode.set_irq_disabled(next);
            prev && !next
        } else {
            timer.mode.set_irq_disabled(true);
            true
        };

        if set_irq {
            match irq.which {
                0 => system.irqctl.stat().set_timer0(true),
                1 => system.irqctl.stat().set_timer1(true),
                2 => system.irqctl.stat().set_timer2(true),
                _ => unimplemented!(),
            }
        }
    }
}

pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> T {
    let offs = addr - PADDR_START;
    let which = (offs >> 4) as usize;
    Timers::update_value(system, which);

    let timer = &mut system.timers[which];

    let v = match offs & 0xF {
        0 => timer.counter(),
        4 => timer.read_mode(),
        8 => timer.target,
        n => unimplemented!("timer read {n}"),
    };

    T::from_u32(u32::from(v))
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, data: T) {
    let offs = addr - PADDR_START;
    let which = (offs >> 4) as usize;
    Timers::update_value(system, which);

    let timer = &mut system.timers[which];
    let data = data.to_u16();

    match offs & 0xF {
        0 => timer.set_counter(data),
        4 => timer.set_mode(data),
        8 => timer.target = data,
        n => unimplemented!("timer write {n}"),
    };
}

impl Index<usize> for Timers {
    type Output = Timer;

    fn index(&self, index: usize) -> &Self::Output {
        &self.timers[index]
    }
}

impl IndexMut<usize> for Timers {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.timers[index]
    }
}

#[derive(Clone, Copy, PartialEq)]
enum SyncMode {
    FreeRun,
    Paused,
    ResetOnHSync,
    HSyncOnly,
    PauseOnHsync,
    ResetOnVSync,
    VSyncOnly,
    PauseOnVsync,
    StartOnNextLine,
    StartOnNextFrame,
}

#[derive(Clone, Copy, PartialEq)]
enum Clock {
    Cpu,
    CpuDiv8,
    Dot,
    HBlank,
}

const CLOCK_SOURCE_MATRIX: [[Clock; 4]; 3] = [
    [Clock::Cpu, Clock::Dot, Clock::Cpu, Clock::Dot],
    [Clock::Cpu, Clock::HBlank, Clock::Cpu, Clock::HBlank],
    [Clock::Cpu, Clock::Cpu, Clock::CpuDiv8, Clock::CpuDiv8],
];

const SYNC_MODE_MATRIX: [[SyncMode; 4]; 3] = [
    [
        SyncMode::PauseOnHsync,
        SyncMode::ResetOnHSync,
        SyncMode::HSyncOnly,
        SyncMode::StartOnNextLine,
    ],
    [
        SyncMode::PauseOnVsync,
        SyncMode::ResetOnVSync,
        SyncMode::VSyncOnly,
        SyncMode::StartOnNextFrame,
    ],
    [
        SyncMode::Paused,
        SyncMode::FreeRun,
        SyncMode::FreeRun,
        SyncMode::Paused,
    ],
];

bitfield::bitfield! {
    #[derive(Default)]
    struct Mode(u16);
    sync_enabled, set_sync_enabled : 0;
    sync_mode, _ : 2, 1;
    reset_to_target, _ : 3;
    irq_target, _ : 5, 4;
    irq_repeat, _: 6;
    irq_toggle, _: 7;
    clock_src, _: 9, 8;
     irq_disabled, set_irq_disabled: 10;
    reached_target, set_reached_target: 11;
    reached_ffff, set_reached_ffff: 12;
}

#[derive(Default)]
pub struct Timer {
    counter: u32,
    mode: Mode,
    target: u16,
    last_read: u64,
}

impl Timer {
    fn counter(&self) -> u16 {
        self.counter as u16
    }

    fn set_counter(&mut self, val: u16) {
        self.counter = u32::from(val)
    }

    fn read_mode(&mut self) -> u16 {
        let v = self.mode.0;
        // Bit 12-11 are reset after read
        self.mode.0 &= !0x1800;
        v
    }

    fn set_mode(&mut self, val: u16) {
        // Reset timer value on mode write
        self.counter = 0;
        // Bit 12-11 are read only
        self.mode.0 = (val & !0x1800) | (self.mode.0 & 0x1800);
        // Bit 10 sets on write
        self.mode.set_irq_disabled(true);
    }

    fn get_ticks_to_value(&mut self, target: u16) -> u32 {
        let counter = self.counter;
        let target = u32::from(target);
        let reset = match self.mode.reset_to_target() {
            true => target,
            false => 0xFFFF,
        };

        if counter <= target {
            target - counter
        } else {
            (reset + 1 - counter) + target
        }
    }
}
