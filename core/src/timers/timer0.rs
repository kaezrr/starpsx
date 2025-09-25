use crate::{System, timers::TimerMode};

#[derive(Default)]
pub struct Timer0 {
    value: u32,
    target: u32,
    last_updated_on: u64,
    mode: TimerMode,

    in_hsync: bool,
}

impl Timer0 {
    pub fn read(system: &mut System, offs: u32) -> u32 {
        Self::update_value(system);
        let timer = &mut system.timers.timer0;

        match offs {
            0 => timer.value,
            4 => timer.mode.0,
            8 => timer.target,
            _ => panic!("invalid timer 0 register {offs}"),
        }
    }

    pub fn write(system: &mut System, offs: u32, val: u32) {
        Self::update_value(system);
        let timer = &mut system.timers.timer0;

        match offs {
            0 => timer.value = val & 0xFFFF,
            4 => Self::write_mode(system, val),
            8 => timer.target = val & 0xFFFF,
            _ => panic!("invalid timer 0 register {offs}"),
        }
    }

    fn write_mode(system: &mut System, val: u32) {
        let timer = &mut system.timers.timer0;

        // Reset timer value on mode write
        timer.value = 0;

        // Bit 12-11 are read only
        timer.mode.0 = val & !0x1800;

        // Bit 10 sets on write
        timer.mode.set_irq_disable(true);
    }

    fn update_value(system: &mut System) {
        let timer = &mut system.timers.timer0;
        let clk_delta = (system.scheduler.sysclk() - timer.last_updated_on) as u32;

        // Update last read cycle
        timer.last_updated_on = system.scheduler.sysclk();

        // Paused timer
        if timer.mode.sync_enable() {
            match timer.mode.sync_mode() {
                0 if timer.in_hsync => return,
                2 | 3 if !timer.in_hsync => return,
                _ => (),
            }
        }

        let reset = match timer.mode.reset_to_target() {
            true => timer.target,
            false => 0xFFFF,
        };

        // Actual number of counter increments
        let delta = match timer.mode.clock_src() {
            0 | 2 => clk_delta,
            1 | 3 => clk_delta / system.gpu.get_dot_clock_divider(),
            _ => unreachable!(),
        };

        timer.value = (timer.value + delta) % (reset + 1);
    }

    pub fn enter_hblank(system: &mut System) {
        Self::update_value(system);
        let timer = &mut system.timers.timer0;

        timer.in_hsync = true;

        if timer.mode.sync_enable() && matches!(timer.mode.sync_mode(), 1 | 2) {
            timer.value = 0;
        }
    }

    pub fn exit_hblank(system: &mut System) {
        Self::update_value(system);
        let timer = &mut system.timers.timer0;

        timer.in_hsync = false;

        if timer.mode.sync_enable() && timer.mode.sync_mode() == 3 {
            timer.mode.set_sync_enable(false);
        }
    }
}
