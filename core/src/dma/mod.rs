pub mod channel;
pub mod utils;

use std::array::from_fn;

use channel::Channel;
use utils::Direction;
use utils::Mode;
use utils::Port;
use utils::Step;

use crate::System;
use crate::mem::ByteAddressable;

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
    fn master_flag(self) -> bool {
        let flags = (self.0 >> 24) & 0x7f;
        let mask = (self.0 >> 16) & 0x7f;
        self.bus_error() || (self.channel_irq_en() && (flags & mask) != 0)
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

    const fn set_channel_irq_flag_bit(&mut self, port: Port) {
        self.0 |= 1 << ((port as u32) + 24);
    }

    const fn mask_enabled(self, port: Port) -> bool {
        self.0 & (1 << ((port as u32) + 16)) != 0
    }
}

pub struct DMAController {
    dpcr: u32,
    dicr: Interrupt,
    pub channels: [Channel; 7],
}

impl Default for DMAController {
    fn default() -> Self {
        Self {
            dpcr: 0x0765_4321,
            dicr: Interrupt(0),
            channels: from_fn(|_| Channel::new()),
        }
    }
}

pub const PADDR_START: u32 = 0x1F80_1080;
pub const PADDR_END: u32 = 0x1F80_1100;

impl DMAController {
    /// Check if channel is master-enabled in DPCR (bit 3 of each channel's 4-bit nibble)
    const fn channel_enabled(&self, port: Port) -> bool {
        let bit = (port as u32) * 4 + 3;
        self.dpcr & (1 << bit) != 0
    }

    fn do_dma(system: &mut System, port: Port) {
        match system.dma.channels[port as usize].ctl.mode() {
            Mode::LinkedList => Self::do_dma_linked_list(system, port),
            Mode::Burst | Mode::Slice => Self::do_dma_block(system, port),
        }

        if system.dma.dicr.should_irq_on_channel_complete(port) {
            system.irqctl.stat().set_dma(true);
        }
    }

    fn write_dpcr<T: ByteAddressable>(&mut self, data: T) {
        self.dpcr = data.to_u32();
    }

    fn write_dicr(&mut self, data: u32) -> bool {
        // bit 31 read only, bits 24 - 30 get reset on sets.
        let old_irq = self.dicr;
        let mut new_irq = Interrupt(data & !(1 << 31) | old_irq.0 & (1 << 31));

        let old_flags = old_irq.channel_irq_flag();
        let new_flags = new_irq.channel_irq_flag();

        new_irq.set_channel_irq_flag(old_flags & !(new_flags));

        self.dicr = new_irq;
        self.dicr.update_irq()
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

        tracing::debug!(target: "dma", ?port, size, "dma");

        let mut addr = base;
        for s in (0..size).rev() {
            let cur_addr = addr & 0x1F_FFFC;
            match dir {
                Direction::ToRam => {
                    let src_word = match port {
                        Port::Otc => match s {
                            0 => 0xFF_FFFF,
                            _ => addr.wrapping_sub(4) & 0x1F_FFFC,
                        },
                        Port::Gpu => system.gpu.read(),
                        Port::Spu => system.spu.dma_read(),
                        Port::CdRom => system.cdrom.read_rddata::<u32>(),
                        Port::MdecOut => system.mdec.response(),
                        _ => todo!("DMA source {port:?}"),
                    };
                    system.ram.write::<u32>(cur_addr, src_word);
                }
                Direction::FromRam => {
                    let src_word = system.ram.read::<u32>(cur_addr);
                    match port {
                        Port::Gpu => system.gpu.gp0(src_word),
                        Port::Spu => system.spu.dma_write(src_word),
                        Port::MdecIn => system.mdec.command_or_param(src_word),
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
        let mut addr = channel.base & 0x1F_FFFC;

        loop {
            let header = system.ram.read::<u32>(addr);
            let size = header >> 24;

            for i in 0..size {
                let data = system.ram.read::<u32>((addr + 4 * (i + 1)) & 0x1F_FFFC);
                system.gpu.gp0(data);
            }

            let next_addr = header & 0xFF_FFFF;
            if next_addr & (1 << 23) != 0 {
                break;
            }
            addr = next_addr & 0x1F_FFFC;
        }

        channel.done();
    }
}

pub fn read<T: ByteAddressable>(system: &System, addr: u32) -> T {
    let channel = (((addr >> 4) & 0xF) - 8) as usize;
    let register = addr & 0xF;

    let data = match addr {
        0x1F80_1080..0x1F80_10F0 => system.dma.channels[channel].read(register),
        0x1F80_10F0 => system.dma.dpcr,
        0x1F80_10F4 => system.dma.dicr.0,
        0x1F80_10F6 => system.dma.dicr.0 >> 16,
        _ => unimplemented!("dma read {addr:x}"),
    };

    T::from_u32(data)
}

pub fn write<T: ByteAddressable>(system: &mut System, addr: u32, data: T) {
    let channel = (((addr >> 4) & 0xF) - 8) as usize;
    let register = addr & 0xF;

    match addr {
        0x1F80_1080..0x1F80_10F0 => {
            let port = Port::from(channel);
            system.dma.channels[channel].write(register, data);

            // OTC only supports backwards transfer to RAM; force control bits
            if port == Port::Otc && register == 8 {
                let ctl = &mut system.dma.channels[channel].ctl;
                ctl.0 &= 0x5100_0000; // keep only enable (bit 24) and trigger (bit 28)
                ctl.0 |= 2; // force step = decrement
            }

            if system.dma.channel_enabled(port) && system.dma.channels[channel].active() {
                DMAController::do_dma(system, port);
            }
        }
        0x1F80_10F0 => system.dma.write_dpcr(data),
        0x1F80_10F4 => {
            if system.dma.write_dicr(data.to_u32()) {
                system.irqctl.stat().set_dma(true);
            }
        }
        0x1F80_10F6 => {
            if system.dma.write_dicr(data.to_u32() << 16) {
                system.irqctl.stat().set_dma(true);
            }
        }
        _ => unimplemented!("dma write {addr:x} = {data:x}"),
    }
}
