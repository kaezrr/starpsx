use arrayvec::ArrayVec;

use crate::cdrom::ResponseType;
use crate::consts::HBLANK_DURATION;
use crate::consts::LINE_DURATION;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TimerInterrupt {
    pub which: usize,
    pub toggle: bool,
}

#[derive(PartialEq, Clone, Eq)]
pub enum Event {
    VBlankStart,
    VBlankEnd,
    HBlankStart,
    HBlankEnd,
    Timer(TimerInterrupt),
    CdromResultIrq(ResponseType),
    SerialSend,
    DsrOff,
    SpuTick,
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
    pub const fn sysclk(&self) -> u64 {
        self.sysclk
    }

    pub const fn advance(&mut self, used_cycles: u64) {
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

    pub fn init_with_events(&mut self) {
        self.schedule(
            Event::VBlankStart,
            LINE_DURATION * 240,
            Some(LINE_DURATION * 263),
        );

        self.schedule(
            Event::VBlankEnd,
            LINE_DURATION * 263,
            Some(LINE_DURATION * 263),
        );

        self.schedule(
            Event::HBlankStart,
            LINE_DURATION - HBLANK_DURATION,
            Some(LINE_DURATION),
        );

        self.schedule(Event::HBlankEnd, LINE_DURATION, Some(LINE_DURATION));

        // SPU clocks at 44100Hz, which translates to 768 cycles
        self.schedule(Event::SpuTick, 768, Some(768));
    }
}
