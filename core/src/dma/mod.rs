pub mod channel;
pub mod utils;

use std::array::from_fn;

use crate::cdrom;
use crate::{System, mem::ByteAddressable};
use channel::Channel;
use tracing::trace;
use utils::{Direction, Port, Step, Sync};

bitfield::bitfield! {
    #[derive(Copy, Clone)]
    struct Interrupt(u32);
    bus_error, _ : 15;
    channel_irq_en, _ : 23;
    channel_irq_flag, set_channel_irq_flag: 30, 24;
    channel_irq_mask, _: 22, 16;
    master_irq, set_master_irq : 31;
}

// Ignoring 0-6 channel irq setting, all irqs happen on completion for now
impl Interrupt {
    fn master_flag(&self) -> bool {
        let channel_match = (self.channel_irq_flag() & self.channel_irq_mask()) > 0;
        self.bus_error() || (self.channel_irq_en() && channel_match)
    }

    fn update_irq(&mut self) -> bool {
        let new_master_flag = self.master_flag();
        let old_master_flag = self.master_irq();

        self.set_master_irq(new_master_flag);

        !old_master_flag && new_master_flag
    }

    fn should_irq_on_channel_complete(&mut self, port: Port) -> bool {
        if self.mask_enabled(port) {
            self.set_channel_irq_flag_bit(port);
        }
        self.update_irq()
    }

    fn set_channel_irq_flag_bit(&mut self, port: Port) {
        self.0 |= 1 << ((port as u32) + 24);
    }

    fn mask_enabled(&mut self, port: Port) -> bool {
        self.0 & (1 << ((port as u32) + 16)) != 0
    }
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
    fn get_mut_channel(&mut self, x: u32) -> &mut Channel {
        &mut self.channels[Port::from(x) as usize]
    }

    fn get_channel(&self, x: u32) -> &Channel {
        &self.channels[Port::from(x) as usize]
    }

    fn do_dma(system: &mut System, port: Port) {
        match system.dma.channels[port as usize].ctl.sync() {
            Sync::LinkedList => DMAController::do_dma_linked_list(system, port),
            _ => DMAController::do_dma_block(system, port),
        }

        if system.dma.interrupt.should_irq_on_channel_complete(port) {
            system.irqctl.stat().set_dma(true);
        }
    }

    fn write_dicr<T: ByteAddressable>(&mut self, data: T) {
        debug_assert_eq!(T::LEN, 4); // word aligned dicr write
        trace!(target:"dma", "dma write to dicr={:08x}", data.to_u32());

        // bit 31 read only, bits 24 - 30 get reset on sets.
        let mut new_irq = Interrupt(data.to_u32() & !(1 << 31));
        let old_irq = self.interrupt;

        let old_flags = old_irq.channel_irq_flag();
        let new_flags = new_irq.channel_irq_flag();

        new_irq.set_channel_irq_flag(old_flags & !(new_flags));
        self.interrupt = new_irq;
    }

    fn do_dma_block(system: &mut System, port: Port) {
        let (step, dir, base, size) = {
            let channel = &mut system.dma.channels[port as usize];
            let step: i32 = match channel.ctl.step() {
                Step::Increment => 4,
                Step::Decrement => -4,
            };
            let size = channel.transfer_size().expect("Should not be none!");
            (step, channel.ctl.dir(), channel.base, size)
        };

        let mut addr = base;

        trace!(target:"dma", ?port, addr, "dma block transfer");

        for s in (0..size).rev() {
            let cur_addr = addr & 0x1FFFFC;
            match dir {
                Direction::ToRam => {
                    let src_word = match port {
                        Port::Otc => match s {
                            0 => 0xFFFFFF,
                            _ => addr.wrapping_sub(4) & 0x1FFFFF,
                        },
                        Port::Gpu => system.gpu.read_reg(0x1F801810), // READ register
                        Port::CdRom => cdrom::read(system, 0x1F801802), // RDDATA register
                        _ => todo!("DMA source {port:?}"),
                    };
                    system.ram.write::<u32>(cur_addr, src_word);
                }
                Direction::FromRam => {
                    let src_word = system.ram.read::<u32>(cur_addr);
                    match port {
                        Port::Gpu => system.gpu.gp0(src_word),
                        Port::Spu => trace!(target:"dma", "dma ignoring transfer from ram to spu"),
                        _ => todo!("DMA destination {port:?}"),
                    }
                }
            }
            addr = addr.wrapping_add_signed(step);
        }
        system.dma.channels[port as usize].done();
    }

    fn do_dma_linked_list(system: &mut System, port: Port) {
        let channel = &mut system.dma.channels[port as usize];

        // only supports ram to gpu linked list dma
        assert_eq!(channel.ctl.dir(), Direction::FromRam);
        assert_eq!(port, Port::Gpu);

        let mut addr = channel.base & 0x1FFFFC;

        trace!(target: "dma", ?port, addr, "dma linked list transfer");

        loop {
            let header = system.ram.read::<u32>(addr);
            let size = header >> 24;

            for i in 0..size {
                let data = system.ram.read::<u32>(addr + 4 * (i + 1));
                system.gpu.gp0(data);
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

pub fn read<T: ByteAddressable>(system: &mut System, addr: u32) -> T {
    let offs = addr - PADDR_START;
    let major = (offs >> 4) & 0x7;
    let minor = (offs) & 0xF;
    let dma = &mut system.dma;

    let data = match (major, minor) {
        (0..=6, 0) => dma.get_channel(major).base,
        (0..=6, 4) => dma.get_channel(major).block_ctl.0,
        (0..=6, 8) => dma.get_channel(major).ctl.0,
        (7, 0) => dma.control,
        (7, 4) => dma.interrupt.0,
        _ => unimplemented!("DMA read {offs:x}"),
    };

    T::from_u32(data)
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, data: T) {
    let offs = addr - PADDR_START;
    let major = (offs >> 4) & 0x7;
    let minor = (offs) & 0xF;
    let dma = &mut system.dma;

    // TODO: dicr bug is not fixed but panics
    match (major, minor) {
        (0..=6, 0) => dma.get_mut_channel(major).base = data.to_u32() & 0xFFFFFF,
        (0..=6, 4) => dma.get_mut_channel(major).block_ctl.0 = data.to_u32(),
        (0..=6, 8) => dma.get_mut_channel(major).ctl.0 = data.to_u32(),
        (7, 0) => dma.control = data.to_u32(),
        (7, 4) => dma.write_dicr(data),
        _ => unimplemented!("DMA write {offs:x} = {data:x}"),
    }

    if let 0..=6 = major {
        let port = Port::from(major);
        if dma.channels[port as usize].active() {
            DMAController::do_dma(system, port);
        }
    }
}
