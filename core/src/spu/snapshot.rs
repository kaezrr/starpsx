use core::fmt;

use super::Spu;
use crate::spu::envelope::AdsrPhase;

impl Spu {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            enabled: self.control.enabled(),
            muted: !self.control.unmuted(),
            main_volume_left: i16_volume_to_percent(self.main_volume.l.0),
            main_volume_right: i16_volume_to_percent(self.main_volume.r.0),
            cd_audio_enabled: self.control.cd_enabled(),
            cd_volume_left: i16_volume_to_percent(self.cd_volume.l),
            cd_volume_right: i16_volume_to_percent(self.cd_volume.r),
            status: self.status(),
            irq_enabled: self.control.irq_enabled(),
            irq_address_actual: self.sound_ram.irq_address as u32,
            voices: std::array::from_fn(|i| {
                let v = &self.voices[i];
                VoiceSnapshot {
                    start_address: (v.start_address / 8) as u16,
                    repeat_address: (v.repeat_address / 8) as u16,
                    current_address: v.current_address / 8,
                    sample_rate: sample_rate_to_hz(v.sample_rate),
                    volume_left: i16_volume_to_percent(v.volume.l.0),
                    volume_right: i16_volume_to_percent(v.volume.r.0),
                    adsr_phase: v.envelope.phase,
                    adsr_volume: i16_volume_to_percent(v.envelope.volume as i16),
                }
            }),

            voice_reverb: std::array::from_fn(|i| self.voices[i].reverb_enabled),
            reverb_base: self.reverb.m_base,
            reverb_curr: self.reverb.current_buffer_addr,
            master_reverb_enable: self.control.reverb_enabled(),
            cd_reverb_enable: self.control.cd_reverb_enabled(),
        }
    }
}

#[derive(Default)]
pub struct Snapshot {
    pub enabled: bool,
    pub muted: bool,
    pub main_volume_left: f32,
    pub main_volume_right: f32,
    pub cd_audio_enabled: bool,
    pub cd_volume_left: f32,
    pub cd_volume_right: f32,
    pub status: u16,
    pub irq_enabled: bool,
    pub irq_address_actual: u32,
    pub voices: [VoiceSnapshot; 24],

    pub voice_reverb: [bool; 24],
    pub reverb_base: usize,
    pub reverb_curr: usize,
    pub master_reverb_enable: bool,
    pub cd_reverb_enable: bool,
}

#[derive(Default)]
pub struct VoiceSnapshot {
    pub start_address: u16,
    pub repeat_address: u16,
    pub current_address: u32,
    pub sample_rate: f32,
    pub volume_left: f32,
    pub volume_right: f32,
    pub adsr_phase: AdsrPhase,
    pub adsr_volume: f32,
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
