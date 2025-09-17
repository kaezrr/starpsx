use crate::memory::Bus;
use crate::memory::ByteAddressable;

impl Bus {
    pub fn gpu_read_handler<T: ByteAddressable>(&mut self, offs: u32) -> T {
        if T::LEN == 1 {
            panic!("unmapped gpu read byte");
        }
        if T::LEN == 2 {
            panic!("unmapped gpu read half word");
        }
        T::from_u32(self.gpu.read_reg(offs))
    }

    pub fn gpu_write_handler<T: ByteAddressable>(&mut self, offs: u32, data: T) {
        if T::LEN == 1 {
            panic!("unmapped gpu write byte");
        }
        if T::LEN == 2 {
            panic!("unmapped gpu write half word");
        }
        self.gpu.write_reg(offs, data.to_u32())
    }

    pub fn dma_read_handler<T: ByteAddressable>(&mut self, offs: u32) -> T {
        if T::LEN == 1 {
            panic!("unmapped dma read byte");
        }
        if T::LEN == 2 {
            panic!("unmapped dma read half word");
        }
        T::from_u32(self.dma.read_reg(offs))
    }

    pub fn dma_write_handler<T: ByteAddressable>(&mut self, offs: u32, data: T) {
        if T::LEN == 1 {
            panic!("unmapped dma write byte");
        }
        if T::LEN == 2 {
            panic!("unmapped dma write half word");
        }
        if let Some(port) = self.dma.write_reg(offs, data.to_u32()) {
            self.dma.do_dma(port, &mut self.ram, &mut self.gpu);
        }
    }

    pub fn irq_read_handler<T: ByteAddressable>(&mut self, offs: u32) -> T {
        if T::LEN == 1 {
            panic!("unmapped irq read byte {offs:08x}");
        }
        T::from_u32(self.irqctl.read_reg(offs))
    }

    pub fn irq_write_handler<T: ByteAddressable>(&mut self, offs: u32, data: T) {
        if T::LEN == 1 {
            panic!("unmapped irq write byte {offs:08x} {data:08x}");
        }
        self.irqctl.write_reg(offs, data.to_u32())
    }
}
