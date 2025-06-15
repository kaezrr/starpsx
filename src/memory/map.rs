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

pub const BIOS: Range = Range(0xBFC00000, 512 * 1024);
