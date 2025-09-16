use crate::dma::utils::Port;
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

    pub fn dma_read_handler<T: ByteAddressable>(&mut self, offs: u32) -> T {
        if T::LEN == 1 {
            panic!("unmapped dma read byte");
        }
        if T::LEN == 2 {
            panic!("unmapped dma read half word");
        }
        T::from_u32(self.dma.read_reg(offs))
    }

    pub fn gpu_write_handler<T: ByteAddressable>(&mut self, offs: u32, data: T) {
        if T::LEN == 1 {
            panic!("unmapped gpu read byte");
        }
        if T::LEN == 2 {
            panic!("unmapped gpu read half word");
        }
        self.gpu.write_reg(offs, data.to_u32())
    }

    pub fn dma_write_handler<T: ByteAddressable>(&mut self, offs: u32, data: T) {
        if T::LEN == 1 {
            panic!("unmapped dma read byte");
        }
        if T::LEN == 2 {
            panic!("unmapped dma read half word");
        }
        if let Some(port) = self.dma.write_reg(offs, data.to_u32()) {
            self.do_dma(port);
        }
    }

    pub fn memctl_write_handler<T: ByteAddressable>(&mut self, offs: u32, data: T) {
        if T::LEN == 1 {
            panic!("unmapped dma read byte");
        }
        if T::LEN == 2 {
            panic!("unmapped dma read half word");
        }
        match offs {
            0 => {
                if data.to_u32() != 0x1F000000 {
                    panic!("Bad expansion 1 base address");
                }
            }
            4 => {
                if data.to_u32() != 0x1F802000 {
                    panic!("Bad expansion 2 base address");
                }
            }
            _ => eprintln!("Unhandled write to MEMCTRL"),
        }
    }
}
