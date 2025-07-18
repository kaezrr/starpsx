pub mod channel;
pub mod utils;

use std::array::from_fn;

use bitfield::bitfield;
use channel::Channel;
use utils::Port;

bitfield! {
    #[derive(Copy, Clone)]
    pub struct Interrupt(u32);
    bus_er, _ : 15;
    irq_en, _ : 23;
    channel_irq_en, _: 30, 24;
    channel_irq_fl, set_channel_irq_fl: 22, 16;
}

pub struct Dma {
    control: u32,
    interrupt: Interrupt,
    pub channels: [Channel; 8],
}

impl Dma {
    pub fn new() -> Self {
        Dma {
            control: 0x07654321,
            interrupt: Interrupt(0),
            channels: from_fn(|_| Channel::new()),
        }
    }

    /// Status of DMA interrupt
    pub fn irq_stat(&self) -> bool {
        let channel_irq = self.interrupt.channel_irq_en() & self.interrupt.channel_irq_fl();
        self.interrupt.bus_er() || (self.interrupt.irq_en() && channel_irq != 0)
    }

    pub fn get_mut_channel(&mut self, x: u32) -> &mut Channel {
        &mut self.channels[Port::from(x) as usize]
    }

    pub fn get_channel(&self, x: u32) -> &Channel {
        &self.channels[Port::from(x) as usize]
    }

    pub fn get_reg(&self, offs: u32) -> u32 {
        let major = (offs >> 4) & 0x7;
        let minor = (offs) & 0xF;

        match (major, minor) {
            (0..=6, 0) => self.get_channel(major).base,
            (0..=6, 4) => self.get_channel(major).block_ctl.0,
            (0..=6, 8) => self.get_channel(major).ctl.0,
            (7, 0) => self.control,
            (7, 4) => self.interrupt.0,
            _ => panic!("Unhandled DMA read {offs:x}"),
        }
    }

    pub fn set_reg(&mut self, offs: u32, data: u32) -> Option<Port> {
        let major = (offs >> 4) & 0x7;
        let minor = (offs) & 0xF;

        match (major, minor) {
            (0..=6, 0) => self.get_mut_channel(major).base = data & 0xFFFFFF,
            (0..=6, 4) => self.get_mut_channel(major).block_ctl.0 = data,
            (0..=6, 8) => self.get_mut_channel(major).ctl.0 = data,
            (7, 0) => self.control = data,
            (7, 4) => {
                let mut data = Interrupt(data);
                data.set_channel_irq_fl(!data.channel_irq_fl());
                self.interrupt.0 = data.0;
            }
            _ => panic!("Unhandled DMA read {offs:x}"),
        }

        match major {
            0..=6 => {
                let port = Port::from(major);
                self.channels[port as usize].active().then_some(port)
            }
            _ => None,
        }
    }
}
