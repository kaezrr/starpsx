use num_enum::FromPrimitive;

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum AdsrPhase {
    #[default]
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Default, FromPrimitive, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
enum Mode {
    #[default]
    Linear = 0,
    Exponential = 1,
}

#[derive(Default, FromPrimitive, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
enum Direction {
    #[default]
    Increase = 0,
    Decrease = 1,
}

bitfield::bitfield! {
    #[derive(Default)]
    pub struct AdsrConfiguration(u32);
    u8, into Mode, sustain_mode, _: 31, 31;
    u8, into Direction, sustain_dir, _ : 30, 30;
    u8, sustain_shift, _: 28, 24;
    u8, sustain_step, _: 23, 22;

    u8, into Mode, release_mode, _ : 21, 21;
    u8, release_shift, _ : 20, 16;

    u8, into Mode, attack_mode, _: 15, 15;
    u8, attack_shift, _: 14, 10;
    u8, attack_step, _: 9, 8;

    u8, decay_shift, _: 7, 4;
    u16, sustain_level, _ :3, 0;
}

#[derive(Default)]
pub struct AdsrEnvelope {
    pub phase: AdsrPhase,
    pub volume: u16,
    pub register: AdsrConfiguration,

    counter: u32,
    counter_reload: u32,
    step: i16,

    exponential: bool,
    decreasing: bool,

    shift: u8,
    step_index: u8,

    sustain_level: u16,
}

impl AdsrEnvelope {
    fn calc(&mut self) {
        const DIRTABLE: [[i16; 4]; 2] = [[7, 6, 5, 4], [-8, -7, -6, -5]];

        let mut step = DIRTABLE[usize::from(self.decreasing)][self.step_index as usize];
        step <<= 11u8.saturating_sub(self.shift);

        let mut counter = 1 << self.shift.saturating_sub(11);

        if self.exponential && !self.decreasing && self.volume > 0x6000 {
            if self.shift < 10 {
                step >>= 2;
            } else if self.shift >= 11 {
                counter <<= 2;
            } else {
                step >>= 2;
                counter <<= 2;
            }
        } else if self.exponential && self.decreasing {
            step = ((i32::from(step) * i32::from(self.volume)) >> 15) as i16;
        }

        self.step = step;
        self.counter_reload = counter.max(1);

        let r = i32::from(self.step_index) | (i32::from(self.shift) << 2);
        if r != 0x7F {
            self.counter_reload = self.counter_reload.min(0x8000);
        }

        self.counter = self.counter_reload;
    }

    fn load_attack(&mut self) {
        self.phase = AdsrPhase::Attack;
        self.decreasing = false;
        self.exponential = self.register.attack_mode() == Mode::Exponential;
        self.shift = self.register.attack_shift();
        self.step_index = self.register.attack_step();
        self.volume = 0;
        self.calc();
    }

    fn load_decay(&mut self) {
        self.phase = AdsrPhase::Decay;
        self.decreasing = true;
        self.exponential = true;
        self.shift = self.register.decay_shift();
        self.step_index = 0;
        self.volume = 0x7FFF;
        self.calc();
    }

    fn load_sustain(&mut self) {
        self.phase = AdsrPhase::Sustain;
        self.decreasing = self.register.sustain_dir() == Direction::Decrease;
        self.exponential = self.register.sustain_mode() == Mode::Exponential;
        self.shift = self.register.sustain_shift();
        self.step_index = self.register.sustain_step();
        self.volume = self.sustain_level;
        self.calc();
    }

    fn load_release(&mut self) {
        self.phase = AdsrPhase::Release;
        self.decreasing = true;
        self.exponential = self.register.release_mode() == Mode::Exponential;
        self.shift = self.register.release_shift();
        self.step_index = 0;
        self.calc();
    }

    pub fn tick(&mut self) {
        if self.phase == AdsrPhase::Off {
            return;
        }

        self.counter -= 1;
        if self.counter > 0 {
            return;
        }

        self.counter = self.counter_reload;
        self.volume = self.volume.saturating_add_signed(self.step);
        self.volume = self.volume.clamp(0, 0x7FFF);

        // Recalculate step sizes for exponential steps
        self.calc();

        match self.phase {
            AdsrPhase::Attack if self.volume >= 0x7FFF => {
                self.load_decay();
            }

            AdsrPhase::Decay if self.volume <= self.sustain_level => {
                self.load_sustain();
            }

            AdsrPhase::Release if self.volume == 0 => {
                self.phase = AdsrPhase::Off;
            }

            _ => {}
        }
    }

    pub fn key_on(&mut self) {
        self.load_attack();
    }

    pub fn key_off(&mut self) {
        self.load_release();
    }

    pub fn update_sustain_level(&mut self) {
        self.sustain_level = (self.register.sustain_level() + 1) * 0x800;
        self.sustain_level = self.sustain_level.min(0x7FFF);
    }
}
