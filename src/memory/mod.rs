pub struct Bus;

impl Bus {
    pub fn read8(&self, addr: u32) -> u8 {
        0
    }
    pub fn read16(&self, addr: u32) -> u16 {
        0
    }
    pub fn read32(&self, addr: u32) -> u32 {
        0
    }

    pub fn write8(&mut self, addr: u32, data: u8) {}

    pub fn write16(&mut self, addr: u32, data: u16) {}

    pub fn write32(&mut self, addr: u32, data: u32) {}
}
