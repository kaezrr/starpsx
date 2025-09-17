pub mod channel;
pub mod utils;

use std::array::from_fn;

use crate::{gpu::Gpu, memory::Ram};
use channel::Channel;
use utils::{Direction, Port, Step, Sync};

bitfield::bitfield! {
    #[derive(Copy, Clone)]
    pub struct Interrupt(u32);
    bus_er, _ : 15;
    irq_en, _ : 23;
    channel_irq_en, _: 30, 24;
    channel_irq_fl, set_channel_irq_fl: 22, 16;
}

pub struct DMAController {
    control: u32,
    interrupt: Interrupt,
    pub channels: [Channel; 8],
}

impl Default for DMAController {
    fn default() -> Self {
        DMAController {
            control: 0x07654321,
            interrupt: Interrupt(0),
            channels: from_fn(|_| Channel::new()),
        }
    }
}

pub const PADDR_START: u32 = 0x1F801080;
pub const PADDR_END: u32 = 0x1F8010FF;

impl DMAController {
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

    pub fn read_reg(&self, addr: u32) -> u32 {
        let offs = addr - PADDR_START;
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

    pub fn write_reg(&mut self, addr: u32, data: u32) -> Option<Port> {
        let offs = addr - PADDR_START;
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

    pub fn do_dma(&mut self, port: Port, ram: &mut Ram, gpu: &mut Gpu) {
        match self.channels[port as usize].ctl.sync() {
            Sync::LinkedList => self.do_dma_linked_list(port, ram, gpu),
            _ => self.do_dma_block(port, ram, gpu),
        }
    }

    fn do_dma_block(&mut self, port: Port, ram: &mut Ram, gpu: &mut Gpu) {
        let (step, dir, base, size) = {
            let channel = &mut self.channels[port as usize];
            let step: i32 = match channel.ctl.step() {
                Step::Increment => 4,
                Step::Decrement => -4,
            };
            let size = channel.transfer_size().expect("Should not be none!");
            (step, channel.ctl.dir(), channel.base, size)
        };

        let mut addr = base;
        for s in (1..=size).rev() {
            let cur_addr = addr & 0x1FFFFC;
            match dir {
                Direction::ToRam => {
                    let src_word = match port {
                        Port::Otc => match s {
                            1 => 0xFFFFFF,
                            _ => addr.wrapping_sub(4) & 0x1FFFFF,
                        },
                        _ => panic!("Unhandled DMA source port"),
                    };
                    ram.write::<u32>(cur_addr, src_word);
                }
                Direction::FromRam => {
                    let src_word = ram.read::<u32>(cur_addr);
                    match port {
                        Port::Gpu => gpu.gp0(src_word),
                        _ => panic!("Unhandled DMA destination port"),
                    }
                }
            }
            addr = addr.wrapping_add_signed(step);
        }
        self.channels[port as usize].done();
    }

    fn do_dma_linked_list(&mut self, port: Port, ram: &mut Ram, gpu: &mut Gpu) {
        let channel = &mut self.channels[port as usize];
        if channel.ctl.dir() == Direction::ToRam {
            panic!("Invalid DMA direction for linked list mode.");
        }
        if port != Port::Gpu {
            panic!("Attempted linked list DMA on port {}", port as usize);
        }

        let mut addr = channel.base & 0x1FFFFC;
        loop {
            let header = ram.read::<u32>(addr);
            let size = header >> 24;

            for i in 0..size {
                let data = ram.read::<u32>(addr + 4 * (i + 1));
                gpu.gp0(data);
            }

            let next_addr = header & 0xFFFFFF;
            if next_addr & (1 << 23) != 0 {
                break;
            }
            addr = next_addr;
        }

        channel.done();
    }
}
