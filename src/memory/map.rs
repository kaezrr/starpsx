pub struct Range(u32, u32);
impl Range {
    pub fn contains(&self, addr: u32) -> Option<u32> {
        if addr >= self.0 && addr < self.0 + self.1 {
            Some(addr - self.0)
        } else {
            None
        }
    }
}

pub const RAM: Range = Range(0x00000000, 2048 * 1024);
pub const BIOS: Range = Range(0x1FC00000, 512 * 1024);
pub const MEMCTL: Range = Range(0x1F801000, 36);
pub const RAMSIZE: Range = Range(0x1F801060, 4);
pub const SPU: Range = Range(0x1F801C00, 640);
pub const CACHECTL: Range = Range(0xFFFE0130, 4);
pub const EXPANSION2: Range = Range(0x1F802000, 66);
pub const EXPANSION1: Range = Range(0x1F000000, 0x100);
pub const IRQCTL: Range = Range(0x1F801070, 8);
pub const TIMERS: Range = Range(0x1F801100, 48);
pub const DMA: Range = Range(0x1F801080, 0x80);
pub const GPU: Range = Range(0x1F801810, 8);

const REGION_MASK: [u32; 8] = [
    0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, // KUSEG
    0x7FFFFFFF, // KSEG0
    0x1FFFFFFF, // KSEG1
    0xFFFFFFFF, 0xFFFFFFFF, // KSEG2
];

pub fn mask_region(addr: u32) -> u32 {
    addr & REGION_MASK[(addr >> 29) as usize]
}
