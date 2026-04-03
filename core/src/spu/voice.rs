use crate::spu::Sweep;
use crate::spu::Volume;

#[derive(Default)]
pub struct Voice {
    pub volume: Volume<Sweep>,
    pub sample_rate: u16,

    /// This register holds the sample start address (not the current address, ie. the register doesn't increment during playback).
    /// Writing to this register has no effect on the currently playing voice.
    /// The start address is copied to the current address upon Key On.
    pub start_address: u16,

    /// If the hardware finds an ADPCM header with Loop-Start-Bit,
    /// then it copies the current address to the repeat addresss register.
    /// If the hardware finds an ADPCM header with Loop-Stop-Bit,
    /// then it copies the repeat addresss register setting to the current address; after playing the current ADPCM block.
    pub repeat_address: u16,

    pub current_address: usize,
}

impl Voice {
    /// (0=stop, 4000h=fastest, 4001h..FFFFh=usually same as 4000h)
    pub fn set_sample_rate(&mut self, val: u16) {
        self.sample_rate = val.min(0x4000);
    }

    pub fn set_start_address(&mut self, val: u16) {
        self.start_address = val;
        self.current_address = usize::from(val) * 8;
    }
}
