mod channel;

use std::array::from_fn;

use bitfield::bitfield;
use channel::Channel;

use crate::dma::channel::Port;

bitfield! {
    #[derive(Copy, Clone)]
    pub struct Interrupt(u32);
    u32;
    bus_er, _ : 15;
    irq_en, _ : 23;
    channel_irq_en, _: 30, 24;
    channel_irq_fl, set_channel_irq_fl: 22, 16;
}

pub struct Dma {
    control: u32,
    interrupt: Interrupt,
    channels: [Channel; 8],
}

impl Dma {
    pub fn new() -> Self {
        Dma {
            control: 0x07654321,
            interrupt: Interrupt(0x00000000),
            channels: from_fn(|_| Channel::new()),
        }
    }

    /// Status of DMA interrupt
    pub fn irq_stat(&self) -> bool {
        let channel_irq = self.interrupt.channel_irq_en() & self.interrupt.channel_irq_fl();
        self.interrupt.bus_er() || (self.interrupt.irq_en() && channel_irq != 0)
    }

    pub fn get_reg(&self, offs: u32) -> u32 {
        match offs {
            0x08 => self.channels[Port::MdecIn as usize].ctl.0,
            0x18 => self.channels[Port::MdecOut as usize].ctl.0,
            0x28 => self.channels[Port::Gpu as usize].ctl.0,
            0x38 => self.channels[Port::CdRom as usize].ctl.0,
            0x48 => self.channels[Port::Spu as usize].ctl.0,
            0x58 => self.channels[Port::Pio as usize].ctl.0,
            0x68 => self.channels[Port::Otc as usize].ctl.0,
            0x70 => self.control,
            0x74 => self.interrupt.0,
            _ => panic!("Unhandled DMA read {offs:x}"),
        }
    }

    pub fn set_reg(&mut self, offs: u32, data: u32) {
        match offs {
            0x08 => self.channels[Port::MdecIn as usize].set(data),
            0x18 => self.channels[Port::MdecOut as usize].set(data),
            0x28 => self.channels[Port::Gpu as usize].set(data),
            0x38 => self.channels[Port::CdRom as usize].set(data),
            0x48 => self.channels[Port::Spu as usize].set(data),
            0x58 => self.channels[Port::Pio as usize].set(data),
            0x68 => self.channels[Port::Otc as usize].set(data),
            0x70 => self.control = data,
            0x74 => {
                let mut data = Interrupt(data);
                data.set_channel_irq_fl(!data.channel_irq_fl());
                self.interrupt.0 = data.0;
            }
            _ => panic!("Unhandled DMA write {offs:x}"),
        }
    }
}
