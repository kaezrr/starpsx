use crate::timer::TimerMode;

#[derive(Default)]
pub struct Timer0 {
    value: u32,
    mode: TimerMode,
    target: u32,
    last_read_cycle: u32,
}

impl Timer0 {
    pub fn read(&mut self, offs: u32) -> u32 {
        match offs {
            0 => self.value,
            4 => self.mode.0,
            8 => self.target,
            _ => panic!("invalid timer 0 register {offs}"),
        }
    }

    pub fn write(&mut self, offs: u32, val: u32) {
        match offs {
            0 => self.value = val,
            4 => self.mode.0 = val,
            8 => self.target = val,
            _ => panic!("invalid timer 0 register {offs}"),
        }
    }
}
