use crate::cdrom::ResponseType;
use arrayvec::ArrayVec;

#[derive(Clone, Copy, PartialEq)]
pub struct TimerInterrupt {
    pub which: usize,
    pub toggle: bool,
}

#[derive(PartialEq, Clone)]
pub enum Event {
    VBlankStart,
    VBlankEnd,
    HBlankStart,
    HBlankEnd,
    Timer(TimerInterrupt),
    CdromResultIrq(ResponseType),
    SerialSend,
}

pub struct Task {
    pub event: Event,
    pub cycle: u64,
    pub repeat: Option<u64>,
}

#[derive(Default)]
pub struct EventScheduler {
    sysclk: u64,
    tasks: ArrayVec<Task, 32>,
}

impl EventScheduler {
    pub fn sysclk(&self) -> u64 {
        self.sysclk
    }

    pub fn advance(&mut self, used_cycles: u64) {
        self.sysclk += used_cycles;
    }

    pub fn get_next_event(&mut self) -> Option<Event> {
        if self.sysclk < self.tasks.first()?.cycle {
            return None;
        }

        let task = self.tasks.remove(0);

        if let Some(cycles) = task.repeat {
            // Reschedule repeating event
            self.schedule(task.event.clone(), cycles, Some(cycles));
        }

        Some(task.event)
    }

    pub fn unschedule(&mut self, event: &Event) {
        self.tasks.retain(|e| e.event != *event);
    }

    pub fn schedule(&mut self, event: Event, cycles_length: u64, repeat: Option<u64>) {
        self.unschedule(&event);

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
