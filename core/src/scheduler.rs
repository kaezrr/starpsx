use arrayvec::ArrayVec;

#[derive(Clone, Copy, PartialEq)]
pub enum Event {
    VBlank,
    HBlank,
}

struct Task {
    pub event: Event,
    pub cycle: u64,
    pub repeat: Option<u64>,
}

#[derive(Default)]
pub struct EventScheduler {
    sysclk: u64,
    tasks: ArrayVec<Task, 2>,
}

impl EventScheduler {
    pub fn get_next_event(&mut self) -> Event {
        let task = self.tasks.remove(0);
        if let Some(cycles) = task.repeat {
            self.subscribe(task.event, cycles, true);
        }
        task.event
    }

    pub fn progress(&mut self, cycles: u64) {
        self.sysclk += cycles
    }

    pub fn cycles_till_next_event(&self) -> u64 {
        self.tasks
            .first()
            .unwrap()
            .cycle
            .saturating_sub(self.sysclk)
    }

    pub fn subscribe(&mut self, event: Event, cycles_length: u64, repeat: bool) {
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
                repeat: repeat.then_some(cycles_length),
            },
        );
    }
}
