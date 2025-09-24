use arrayvec::ArrayVec;

#[derive(Clone, Copy, PartialEq)]
pub enum Event {
    VBlank,
    HBlank,
}

#[derive(Default)]
pub struct EventScheduler {
    sysclk: u64,
    events: ArrayVec<(Event, u64), 3>,
}

impl EventScheduler {
    pub fn get_next_event(&mut self) -> Event {
        self.events.remove(0).0
    }

    pub fn progress(&mut self, cycles: u64) {
        self.sysclk += cycles
    }

    pub fn cycles_till_next_event(&self) -> u64 {
        self.events.first().unwrap().1.saturating_sub(self.sysclk)
    }

    pub fn schedule_event(&mut self, event_type: Event, cycles: u64) {
        self.events.retain(|e| e.0 != event_type);

        let target_cycle = self.sysclk + cycles;
        let pos = self
            .events
            .iter()
            .position(|e| e.1 > target_cycle)
            .unwrap_or(self.events.len());

        self.events.insert(pos, (event_type, target_cycle));
    }
}
