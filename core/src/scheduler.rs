#[derive(Clone, Copy, PartialEq)]
pub enum EventType {
    VBlank,
    HBlank,
}

pub struct Event {
    event_type: EventType,
    target_cycle: u64,
}

#[derive(Default)]
pub struct EventScheduler {
    sysclk: u64,
    events: Vec<Event>,
}

impl EventScheduler {
    pub fn get_next_event(&mut self) -> EventType {
        self.events.remove(0).event_type
    }

    pub fn progress(&mut self, cycles: u64) {
        self.sysclk += cycles
    }

    pub fn cycles_till_next_event(&self) -> u64 {
        self.events
            .first()
            .unwrap()
            .target_cycle
            .saturating_sub(self.sysclk)
    }

    pub fn schedule_event(&mut self, event_type: EventType, cycles: u64) {
        self.events.retain(|e| e.event_type != event_type);

        let target_cycle = self.sysclk + cycles;
        let pos = self
            .events
            .iter()
            .position(|e| e.target_cycle > target_cycle)
            .unwrap_or(self.events.len());

        self.events.insert(
            pos,
            Event {
                event_type,
                target_cycle,
            },
        );
    }
}
