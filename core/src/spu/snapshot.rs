use core::fmt;

use super::Spu;

impl Spu {
    pub fn snapshot(&self) -> Snapshot {
        // Snapshot {
        //     enabled: self.control.enabled(),
        //     muted: !self.control.unmuted(),
        //     main_volume_left: i16_volume_to_percent(self.main_volume.l.volume()),
        //     main_volume_right: i16_volume_to_percent(self.main_volume.r.volume()),
        //     voices: std::array::from_fn(|i| {
        //         let v = &self.voices[i];
        //         VoiceSnapshot {
        //             start_address: (v.start_address >> 3) as u16,
        //             repeat_address: (v.repeat_address >> 3) as u16,
        //             current_address: v.current_address >> 3,
        //             sample_rate: sample_rate_to_hz(v.sample_rate),
        //             volume_left: i16_volume_to_percent(v.volume.l.volume()),
        //             volume_right: i16_volume_to_percent(v.volume.r.volume()),
        //             adsr_phase: v.envelope.phase(),
        //             adsr_volume: i16_volume_to_percent(v.envelope.volume()),
        //         }
        //     }),
        // }
        Snapshot::default()
    }
}

#[derive(Default)]
pub struct Snapshot {
    pub enabled: bool,
    pub muted: bool,
    pub main_volume_left: f32,
    pub main_volume_right: f32,
    pub voices: [VoiceSnapshot; 24],
}

#[derive(Default)]
pub struct VoiceSnapshot {
    pub start_address: u16,
    pub repeat_address: u16,
    pub current_address: usize,
    pub sample_rate: f32,
    pub volume_left: f32,
    pub volume_right: f32,
    pub adsr_phase: AdsrPhase,
    pub adsr_volume: f32,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
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
            Self::Off => write!(f, "Off"),
            Self::Attack => write!(f, "Attack"),
            Self::Decay => write!(f, "Decay"),
            Self::Sustain => write!(f, "Sustain"),
            Self::Release => write!(f, "Release"),
        }
    }
}

fn i16_volume_to_percent(v: i16) -> f32 {
    (f32::from(v) / 0x7FFF as f32) * 100.0
}

fn sample_rate_to_hz(raw: u16) -> f32 {
    f32::from(raw) / 4096.0 * 44100.0
}
