use std::fmt;

use num_enum::FromPrimitive;

#[derive(Default, PartialEq, Clone, Copy, FromPrimitive)]
#[repr(u8)]
enum Direction {
    #[default]
    Increasing = 0,
    Decreasing = 1,
}

#[derive(Default, PartialEq, Clone, Copy, FromPrimitive)]
#[repr(u8)]
enum ChangeRate {
    #[default]
    Linear = 0,
    Exponential = 1,
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum AdsrPhase {
    #[default]
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl fmt::Display for AdsrPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdsrPhase::Off => write!(f, "Off"),
            AdsrPhase::Attack => write!(f, "Attack"),
            AdsrPhase::Decay => write!(f, "Decay"),
            AdsrPhase::Sustain => write!(f, "Sustain"),
            AdsrPhase::Release => write!(f, "Release"),
        }
    }
}

const ENVELOPE_COUNTER_MAX: u32 = 0x8000;

#[derive(Default)]
pub struct AdsrEnvelope {
    pub register: AdsrRegister,

    level: i16,
    phase: AdsrPhase,
    counter: u32,
}

impl AdsrEnvelope {
    pub fn volume(&self) -> i16 {
        self.level
    }

    pub fn phase(&self) -> AdsrPhase {
        self.phase
    }

    pub fn set_volume(&mut self, v: i16) {
        self.level = v;
    }

    pub fn key_on(&mut self) {
        self.level = 0;
        self.counter = 0;
        self.phase = AdsrPhase::Attack;
    }

    pub fn key_off(&mut self) {
        self.phase = AdsrPhase::Release;
    }

    pub fn tick(&mut self) {
        self.check_for_phase_transition();

        if self.phase == AdsrPhase::Off {
            return;
        }

        let (direction, rate, shift, step_value) = self.phase_params();

        let mut counter_increment: u32 = ENVELOPE_COUNTER_MAX >> shift.saturating_sub(11);
        let mut halve_step_twice = false;

        if direction == Direction::Increasing
            && rate == ChangeRate::Exponential
            && self.level > 0x6000
        {
            match shift {
                0..=9 => {
                    halve_step_twice = true;
                }
                10 => {
                    halve_step_twice = true;
                    counter_increment >>= 2;
                }
                _ => {
                    counter_increment >>= 2;
                }
            }
        }

        let combined_rate = (step_value as u16) | ((shift as u16) << 2);
        if combined_rate != 0x7F {
            counter_increment = counter_increment.max(1);
        }

        self.counter = self.counter.wrapping_add(counter_increment);

        if self.counter & ENVELOPE_COUNTER_MAX == 0 {
            return;
        }

        self.counter &= !ENVELOPE_COUNTER_MAX;

        let mut adsr_step: i32 = i32::from(7 - step_value);

        if direction == Direction::Decreasing {
            adsr_step = !adsr_step;
        }

        adsr_step <<= 11u8.saturating_sub(shift);

        let current_level = i32::from(self.level);

        if direction == Direction::Decreasing && rate == ChangeRate::Exponential {
            adsr_step = (adsr_step * current_level) >> 15;
        }

        if halve_step_twice {
            adsr_step >>= 2;
        }

        self.level = (current_level + adsr_step).clamp(0, 0x7FFF) as i16;
    }

    fn check_for_phase_transition(&mut self) {
        match self.phase {
            AdsrPhase::Attack if self.level == 0x7FFF => {
                self.phase = AdsrPhase::Decay;
            }
            AdsrPhase::Decay
                if i32::from(self.level)
                    <= i32::from((self.register.sustain_level_raw() + 1) * 0x800) =>
            {
                self.phase = AdsrPhase::Sustain;
            }
            AdsrPhase::Release if self.level <= 0 => {
                self.level = 0;
                self.phase = AdsrPhase::Off;
            }
            _ => {}
        }
    }

    /// Return (direction, rate, shift, step) for current phase
    fn phase_params(&self) -> (Direction, ChangeRate, u8, u8) {
        let reg = &self.register;
        match self.phase {
            AdsrPhase::Attack => (
                Direction::Increasing,
                reg.attack_mode(),
                reg.attack_shift(),
                reg.attack_step(),
            ),
            AdsrPhase::Decay => (
                Direction::Decreasing,
                ChangeRate::Exponential,
                reg.decay_shift(),
                0,
            ),
            AdsrPhase::Sustain => (
                reg.sustain_direction(),
                reg.sustain_mode(),
                reg.sustain_shift(),
                reg.sustain_step(),
            ),
            AdsrPhase::Release => (
                Direction::Decreasing,
                reg.release_mode(),
                reg.release_shift(),
                0,
            ),
            AdsrPhase::Off => unreachable!("tick() must not be called in Off phase"),
        }
    }
}

bitfield::bitfield! {
    #[derive(Default)]
    pub struct AdsrRegister(u32);
    u8, into ChangeRate, attack_mode, _: 15, 15;
    u8, attack_shift, _: 14, 10;
    u8, attack_step, _: 9, 8;
    u8, decay_shift, _: 7, 4;
    u16, sustain_level_raw, _: 3, 0;
    u8, into ChangeRate, sustain_mode, _: 31, 31;
    u8, into Direction, sustain_direction, _: 30, 30;
    u8, sustain_shift, _: 28, 24;
    u8, sustain_step, _: 23, 22;
    u8, into ChangeRate, release_mode, _: 21, 21;
    u8, release_shift, _: 20, 16;
}

#[expect(unused)]
#[derive(Default, PartialEq, Clone, Copy, FromPrimitive)]
#[repr(u8)]
enum SweepPhase {
    #[default]
    Positive = 0,
    Negative = 1,
}

#[derive(Default, PartialEq, Clone, Copy, FromPrimitive, Debug)]
#[repr(u8)]
enum Mode {
    #[default]
    Fixed = 0,
    Sweep = 1,
}

#[derive(Default)]
pub struct SweepVolume {
    pub register: VolumeRegister,

    level: i16,
    #[expect(unused)]
    counter: u32,
}

impl SweepVolume {
    pub fn volume(&self) -> i16 {
        self.level
    }

    pub fn set_volume(&mut self, v: u16) {
        self.register.0 = v;
        self.level = ((self.register.volume() << 1) >> 1) * 2;

        debug_assert_eq!(self.register.mode(), Mode::Fixed);
    }
}

bitfield::bitfield! {
    #[derive(Default)]
    pub struct VolumeRegister(u16);
    u8, into Mode, mode, _ : 15, 15;
    i16, volume, _ : 14, 0;

    u8, into ChangeRate, sweep_mode, _: 14, 14;
    u8, into Direction, sweep_direction, _: 13, 13;
    u8, into SweepPhase, sweep_phase, _: 12, 12;
    u8, sweep_shift, _ : 6, 2;
    u8, sweep_step, _ : 1, 0;
}
