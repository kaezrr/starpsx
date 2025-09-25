use crate::{System, timers::TimerMode};

#[derive(Default)]
pub struct Timer2 {
    value: u32,
    target: u32,
    last_updated_on: u64,
    pub mode: TimerMode,
}

impl Timer2 {
    pub fn read(system: &mut System, offs: u32) -> u32 {
        Self::update_value(system);
        let timer = &mut system.timers.timer2;

        match offs {
            0 => timer.value,
            4 => timer.mode.0,
            8 => timer.target,
            _ => panic!("invalid timer 0 register {offs}"),
        }
    }

    pub fn write(system: &mut System, offs: u32, val: u32) {
        Self::update_value(system);
        let timer = &mut system.timers.timer2;

        // println!("TIMER2 WRITE {offs} -> {val}");
        match offs {
            0 => timer.value = val,
            4 => Timer2::write_mode(system, val),
            8 => timer.target = val,
            _ => panic!("invalid timer 0 register {offs}"),
        }
    }

    fn write_mode(system: &mut System, val: u32) {
        let timer = &mut system.timers.timer2;

        // Reset timer value on mode write
        timer.value = 0;

        // Bit 12-11 are read only
        timer.mode.0 = val & !0x1800;

        // Bit 10 sets on write
        timer.mode.set_irq_disable(true);
    }

    fn update_value(system: &mut System) {
        let timer = &mut system.timers.timer2;
        let clk_delta = (system.scheduler.sysclk() - timer.last_updated_on) as u32;

        // Update last read cycle
        timer.last_updated_on = system.scheduler.sysclk();

        // Paused timer
        if timer.mode.sync_enable() && matches!(timer.mode.sync_mode(), 0 | 3) {
            return;
        }

        let divisor = match timer.mode.clock_src() {
            0 | 1 => 1,
            2 | 3 => 8,
            _ => unreachable!(),
        };

        // Actual number of counter increments
        let delta = clk_delta / divisor;
        timer.value = (timer.value + delta) % (0xFFFF + 1);
    }
}
