mod snapshot;

pub use snapshot::AdsrPhase;
pub use snapshot::Snapshot;
pub use snapshot::VoiceSnapshot;
use tracing::warn;

use crate::System;

pub const PADDR_START: u32 = 0x1F80_1C00;
pub const PADDR_END: u32 = 0x1F80_1E80;

#[derive(Default)]
pub struct Spu {}

impl Spu {
    pub fn dma_read(&mut self) -> u32 {
        warn!("SPU DMA READ");
        0
    }

    pub fn dma_write(&mut self, word: u32) {
        warn!("SPU DMA WRITE");
    }

    pub fn tick(system: &mut System) -> [i16; 2] {
        [0, 0]
    }
}

#[allow(clippy::match_same_arms)]
/// 8bit, 16bit and 32bit reads are supported
pub fn read<const WIDTH: usize>(system: &System, addr: u32) -> u32 {
    let spu = &system.spu;

    match addr {
        x => unimplemented!("spu read {x:8X}, width={}", WIDTH * 8),
    }
}

#[allow(clippy::match_same_arms)]
///  16bit writes are suppored,
pub fn write<const WIDTH: usize>(system: &mut System, addr: u32, val: u32) {
    let spu = &mut system.spu;

    //  32bit writes are also supported but seem to be particularly unstable
    //  So they are split into 2 16bit writes instead
    if WIDTH == 4 {
        write::<2>(system, addr, val);
        write::<2>(system, addr, val);
        return;
    }

    //  8bit writes to ODD addresses are simply ignored
    //  8bit writes to EVEN addresses are executed as 16bit writes
    if WIDTH == 1 {
        if addr & 1 == 0 {
            write::<2>(system, addr, val);
        }
        return;
    }

    match addr {
        x => unimplemented!("spu write {x:8X}"),
    }
}

pub fn signed4bit(v: u8) -> i32 {
    i32::from((v as i8) << 4 >> 4)
}

fn write_half<const HIGH: bool>(reg: &mut u32, val: u16) {
    let shift = if HIGH { 16 } else { 0 };
    let mask = 0xFFFF << shift;

    *reg = (*reg & !mask) | (u32::from(val) << shift);
}
