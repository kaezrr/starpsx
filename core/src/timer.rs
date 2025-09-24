pub const PADDR_START: u32 = 0x1F801100;
pub const PADDR_END: u32 = 0x1F80112F;

#[derive(Default)]
pub struct Timer {
    timer_values: [u32; 3],
    timer_modes: [u32; 3],
    timer_targets: [u32; 3],
}

impl Timer {
    pub fn read_reg(&mut self, addr: u32) -> u32 {
        let addr = addr - PADDR_START;
        let t = (addr / 0x10) as usize;
        let offs = addr % 0x10;

        match offs {
            0 => self.timer_values[t],
            4 => self.timer_modes[t],
            8 => self.timer_targets[t],
            _ => panic!("unknown timer register {offs} read"),
        }
    }

    pub fn write_reg(&mut self, addr: u32, val: u32) {
        let addr = addr - PADDR_START;
        let t = (addr / 0x10) as usize;
        let offs = addr % 0x10;

        match offs {
            0 => self.timer_values[t] = val,
            4 => self.timer_modes[t] = val,
            8 => self.timer_targets[t] = val,
            _ => panic!("unknown timer register {offs} write"),
        };
    }
}
