use crate::System;
use crate::memory::Bus;
use crate::memory::ByteAddressable;

impl Bus {
    pub fn gpu_read_handler<T: ByteAddressable>(system: &mut System, offs: u32) -> T {
        if T::LEN == 1 {
            panic!("unmapped gpu read byte");
        }
        if T::LEN == 2 {
            panic!("unmapped gpu read half word");
        }
        T::from_u32(system.gpu.read_reg(offs))
    }

    pub fn gpu_write_handler<T: ByteAddressable>(system: &mut System, offs: u32, data: T) {
        if T::LEN == 1 {
            panic!("unmapped gpu write byte");
        }
        if T::LEN == 2 {
            panic!("unmapped gpu write half word");
        }
        system.gpu.write_reg(offs, data.to_u32())
    }

    pub fn dma_read_handler<T: ByteAddressable>(system: &mut System, offs: u32) -> T {
        if T::LEN == 1 {
            panic!("unmapped dma read byte");
        }
        if T::LEN == 2 {
            panic!("unmapped dma read half word");
        }
        T::from_u32(system.dma.read_reg(offs))
    }

    pub fn dma_write_handler<T: ByteAddressable>(system: &mut System, offs: u32, data: T) {
        if T::LEN == 1 {
            panic!("unmapped dma write byte");
        }
        if T::LEN == 2 {
            panic!("unmapped dma write half word");
        }
        if let Some(port) = system.dma.write_reg(offs, data.to_u32()) {
            system
                .dma
                .do_dma(port, &mut system.bus.ram, &mut system.gpu);
        }
    }

    pub fn irq_read_handler<T: ByteAddressable>(system: &mut System, offs: u32) -> T {
        if T::LEN == 1 {
            panic!("unmapped irq read byte");
        }
        T::from_u32(system.irqctl.read_reg(offs))
    }

    pub fn irq_write_handler<T: ByteAddressable>(system: &mut System, offs: u32, data: T) {
        if T::LEN == 1 {
            panic!("unmapped irq write byte");
        }
        system.irqctl.write_reg(offs, data.to_u32())
    }

    pub fn timer_read_handler<T: ByteAddressable>(system: &mut System, offs: u32) -> T {
        if T::LEN == 1 {
            panic!("unmapped timer read byte");
        }
        T::from_u32(system.timer.read_reg(offs))
    }

    pub fn timer_write_handler<T: ByteAddressable>(system: &mut System, offs: u32, data: T) {
        if T::LEN == 1 {
            panic!("unmapped timer write byte");
        }
        system.timer.write_reg(offs, data.to_u32())
    }
}
