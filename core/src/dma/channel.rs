use super::utils::Direction;
use super::utils::Mode;
use super::utils::Step;
use crate::mem::ByteAddressable;

bitfield::bitfield! {
    pub struct Control(u32);
    enabled, set_enabled : 24;
    forced, set_forced: 28;
    pub u8, into Direction, dir, _ : 0, 0;
    pub u8, into Step, step, _ : 1, 1;
    pub u8, into Mode, mode, _ : 10, 9;
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
        let trigger = match self.ctl.mode() {
            Mode::Burst => self.ctl.forced(),
            _ => true,
        };
        self.ctl.enabled() && trigger
    }

    /// Get DMA transfer size in words
    pub fn transfer_size(&self) -> Option<u32> {
        let bs = self.block_ctl.block_size();
        let bc = self.block_ctl.block_count();

        match self.ctl.mode() {
            Mode::Burst => Some(if bs == 0 { 0x10000 } else { bs }),
            Mode::Slice => Some(bc.max(1) * bs),
            Mode::LinkedList => None,
        }
    }

    /// Set the channel status to "completed" state
    pub fn done(&mut self) {
        self.ctl.set_enabled(false);
        self.ctl.set_forced(false);
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
            0 => self.base = data & 0xFF_FFFF,
            4 => self.block_ctl.0 = data,
            8 => self.ctl.0 = data,
            _ => unimplemented!("channel reg write {reg}"),
        }
    }
}
