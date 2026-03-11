use crate::mem::ByteAddressable;

use super::utils::Direction;
use super::utils::Step;
use super::utils::Sync;

bitfield::bitfield! {
    pub struct Control(u32);
    enable, set_enable : 24;
    trigger, set_trigger: 28;
    pub u8, into Direction, dir, _ : 0, 0;
    pub u8, into Step, step, _ : 1, 1;
    pub u8, into Sync, sync, _ : 10, 9;
    chop, _ : 2;
    chop_dma_size, _: 18, 16;
    chop_cpu_size, _: 22, 20;
}

bitfield::bitfield! {
    pub struct Block(u32);
    block_size, _ : 15, 0;
    block_count, _ : 31, 16;
}

pub struct Channel {
    pub ctl: Control,
    pub base: u32,
    pub block_ctl: Block,
}

impl Channel {
    pub fn new() -> Self {
        Channel {
            ctl: Control(0),
            block_ctl: Block(0),
            base: 0,
        }
    }

    pub fn active(&self) -> bool {
        let trigger = match self.ctl.sync() {
            Sync::Manual => self.ctl.trigger(),
            _ => true,
        };
        self.ctl.enable() && trigger
    }

    /// Get DMA transfer size in words
    pub fn transfer_size(&self) -> Option<u32> {
        let bs = self.block_ctl.block_size();
        let bc = self.block_ctl.block_count();

        match self.ctl.sync() {
            Sync::Manual => Some(if bs == 0 { 0x10000 } else { bs }),
            Sync::Request => Some(bc * bs),
            Sync::LinkedList => None,
        }
    }

    /// Set the channel status to "completed" state
    pub fn done(&mut self) {
        self.ctl.set_enable(false);
        self.ctl.set_trigger(false);
    }

    pub fn read(&self, reg: u32) -> u32 {
        match reg {
            0 => self.base,
            4 => self.block_ctl.0,
            8 => self.ctl.0,
            _ => unimplemented!("channel reg read {reg}"),
        }
    }

    pub fn write<T: ByteAddressable>(&mut self, reg: u32, data: T) {
        assert_eq!(T::LEN, 4);

        let data = data.to_u32();
        match reg {
            0 => self.base = data,
            4 => self.block_ctl.0 = data,
            8 => self.ctl.0 = data,
            _ => unimplemented!("channel reg write {reg}"),
        }
    }
}
