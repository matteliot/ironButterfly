//! A clean amp channel: high-pass, a peak compressor and a soft tube edge
//! (`tanh(drive·x·comp)·0.7`). Any EQ after it is up to the pedal.

use super::dsp::{Biquad, coef, tanh};

/// How fast the compressor clamps down.
pub enum Attack {
    /// A fixed per-sample coefficient (0.05 is nearly instant).
    PerSample(f32),
    /// A time constant (s).
    Time(f32),
}

pub struct CleanSettings {
    pub hp_hz: f32,
    pub drive: f32,
    pub attack: Attack,
    /// Compressor release (s).
    pub release: f32,
}

pub struct CleanAmp {
    drive: f32,
    comp: f32,
    c_att: f32,
    c_rel: f32,
    hp: Biquad,
}

impl CleanAmp {
    pub const NEW: Self = Self { drive: 1.0, comp: 1.0, c_att: 1.0, c_rel: 1.0, hp: Biquad::NEW };

    pub fn init(&mut self, s: &CleanSettings) {
        self.drive = s.drive;
        self.c_att = match s.attack {
            Attack::PerSample(c) => c,
            Attack::Time(t) => coef(t),
        };
        self.c_rel = coef(s.release);
        self.hp.highpass(s.hp_hz, 0.707);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let x = self.hp.process(x);
        let a = x.abs() * self.drive;
        let target = if a > 0.5 { 0.5 / a } else { 1.0 };
        self.comp += (target - self.comp) * if target < self.comp { self.c_att } else { self.c_rel };
        tanh(x * self.drive * self.comp) * 0.7
    }
}
