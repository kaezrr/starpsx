pub const PADDR_START: u32 = 0x1F801070;
pub const PADDR_END: u32 = 0x1F801077;

bitfield::bitfield! {
    #[derive(Clone, Copy, Default)]
    pub struct IStat(u32);
    _, set_vblank: 0;
    _, set_gpu: 1;
    _, set_cdrom: 2;
    _, set_dma: 3;
    _, set_timer0: 4;
    _, set_timer1: 5;
    _, set_timer2: 6;
    _, set_ctl_mem: 7;
    _, set_sio: 8;
    _, set_spu: 9;
    _, set_ctl_light: 10;
}

#[derive(Default)]
pub struct InterruptController {
    mask: u32,
    stat: IStat,
}

impl InterruptController {
    pub fn read_reg(&self, addr: u32) -> u32 {
        let offs = addr - PADDR_START;
        println!("READ TO IRQCTL: {offs}");
        match offs {
            0 => self.stat.0,
            4 => self.mask,
            _ => panic!("unknown irqctl register {offs}"),
        }
    }

    pub fn write_reg(&mut self, addr: u32, val: u32) {
        let offs = addr - PADDR_START;
        println!("WRITE TO IRQCTL: {offs} {val:08x}");
        match offs {
            0 => self.stat.0 &= val,
            4 => self.mask = val,
            _ => panic!("unknown irqctl register {offs}"),
        };
    }

    pub fn pending(&self) -> bool {
        self.stat.0 & self.mask != 0
    }

    pub fn stat(&mut self) -> &mut IStat {
        &mut self.stat
    }
}
