use bitfield::bitfield;

bitfield! {
    pub struct Control(u32);
    u32;
    enable, _ : 24;
    trigger, _: 28;
    direction, _ : 0;
    step, _ : 1;
    sync, _ : 10, 9;
    chop, _ : 2;
    chop_dma_size, _: 18, 16;
    chop_cpu_size, _: 22, 20;
}

pub enum Direction {
    ToRam,
    FromRam,
}

pub enum Step {
    Increment,
    Decrement,
}

pub enum Sync {
    Manual,
    Request,
    LinkedList,
}

#[repr(usize)]
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

pub struct Channel {
    pub ctl: Control,
    dir: Direction,
    step: Step,
    sync: Sync,
}

impl Channel {
    pub fn new() -> Self {
        Channel {
            dir: Direction::ToRam,
            sync: Sync::Manual,
            step: Step::Increment,
            ctl: Control(0),
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
}
