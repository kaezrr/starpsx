use arrayvec::ArrayVec;

#[derive(Clone, Copy, PartialEq)]
pub struct TimerInterrupt {
    pub which: usize,
    pub toggle: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DevicePort {
    Gamepad,
    MemoryCard,
}

#[derive(Clone, Copy, PartialEq)]
pub struct SerialSend {
    pub port: DevicePort,
    pub data: u8,
}

impl SerialSend {
    pub fn new(port: u8, data: u8) -> Self {
        Self {
            port: match port {
                0x01 => DevicePort::Gamepad,
                0x81 => DevicePort::MemoryCard,
                _ => unimplemented!("Unknown device port {port:02x}"),
            },
            data,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Event {
    VBlankStart,
    VBlankEnd,
    HBlankStart,
    HBlankEnd,
    Timer(TimerInterrupt),
    Serial(SerialSend),
}

pub struct Task {
    pub event: Event,
    pub cycle: u64,
    pub repeat: Option<u64>,
}

#[derive(Default)]
pub struct EventScheduler {
    sysclk: u64,
    tasks: ArrayVec<Task, 6>,
}

impl EventScheduler {
    pub fn sysclk(&self) -> u64 {
        self.sysclk
    }

    pub fn pop_next_event(&mut self) -> Event {
        let task = self.tasks.remove(0);
        self.sysclk = task.cycle;

        if let Some(cycles) = task.repeat {
            self.schedule(task.event, cycles, Some(cycles));
        }

        task.event
    }

    pub fn cycles_till_next_event(&self) -> u64 {
        self.tasks
            .first()
            .unwrap()
            .cycle
            .saturating_sub(self.sysclk)
    }

    pub fn schedule(&mut self, event: Event, cycles_length: u64, repeat: Option<u64>) {
        self.tasks.retain(|e| e.event != event);

        let cycle = self.sysclk + cycles_length;

        let pos = self
            .tasks
            .iter()
            .position(|e| e.cycle > cycle)
            .unwrap_or(self.tasks.len());

        self.tasks.insert(
            pos,
            Task {
                event,
                cycle,
                repeat,
            },
        );
    }
}
