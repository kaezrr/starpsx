use bitfield::bitfield;

bitfield! {
    pub struct Control(u32);
    u32;
    enable, set_enable : 24;
    trigger, set_trigger: 28;
    direction, _ : 0;
    step, _ : 1;
    sync, _ : 10, 9;
    chop, _ : 2;
    chop_dma_size, _: 18, 16;
    chop_cpu_size, _: 22, 20;
}

bitfield! {
    pub struct Block(u32);
    u32;
    block_size, _ : 15, 0;
    block_count, _ : 31, 16;
}

#[derive(Clone, Copy)]
pub enum Direction {
    ToRam,
    FromRam,
}

#[derive(Clone, Copy)]
pub enum Step {
    Increment,
    Decrement,
}

#[derive(Clone, Copy)]
pub enum Sync {
    Manual,
    Request,
    LinkedList,
}

#[repr(usize)]
#[derive(Clone, Copy)]
pub enum Port {
    // Macroblock decoder input
    MdecIn,
    // Macroblock decoder output
    MdecOut,
    // Graphics Processing Unit
    Gpu,
    // CD-ROM Drive
    CdRom,
    // Sound Processing Unit
    Spu,
    // Extension Port
    Pio,
    // Clear ordering table
    Otc,
}

impl Port {
    pub fn from(index: u32) -> Self {
        match index {
            0 => Port::MdecIn,
            1 => Port::MdecOut,
            2 => Port::Gpu,
            3 => Port::CdRom,
            4 => Port::Spu,
            5 => Port::Pio,
            6 => Port::Otc,
            _ => panic!("Unknown port {index}"),
        }
    }
}

pub struct Channel {
    pub ctl: Control,
    pub base: u32,
    pub block_ctl: Block,
    pub dir: Direction,
    pub step: Step,
    pub sync: Sync,
}

impl Channel {
    pub fn new() -> Self {
        Channel {
            dir: Direction::ToRam,
            sync: Sync::Manual,
            step: Step::Increment,
            ctl: Control(0),
            block_ctl: Block(0),
            base: 0,
        }
    }

    pub fn set(&mut self, data: u32) {
        let data = Control(data);

        self.dir = match data.direction() {
            true => Direction::FromRam,
            false => Direction::ToRam,
        };

        self.step = match data.step() {
            true => Step::Decrement,
            false => Step::Increment,
        };

        self.sync = match data.sync() {
            0 => Sync::Manual,
            1 => Sync::Request,
            2 => Sync::LinkedList,
            n => panic!("Unknown DMA sync mode {n}"),
        };

        self.ctl.0 = data.0;
    }

    pub fn active(&self) -> bool {
        let trigger = match self.sync {
            Sync::Manual => self.ctl.trigger(),
            _ => true,
        };
        self.ctl.enable() && trigger
    }

    /// Get DMA transfer size in words
    pub fn transfer_size(&self) -> Option<u32> {
        let bs = self.block_ctl.block_size();
        let bc = self.block_ctl.block_count();

        match self.sync {
            Sync::Manual => Some(bs),
            Sync::Request => Some(bc * bs),
            Sync::LinkedList => None,
        }
    }

    /// Set the channel status to "completed" state
    pub fn done(&mut self) {
        self.ctl.set_enable(false);
        self.ctl.set_trigger(false);
    }
}
