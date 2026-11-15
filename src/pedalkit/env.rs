//! Envelope followers.

use super::dsp::{coef, exp_hq};

/// Peak follower with one-pole attack and release (per-sample coefficients
/// from `dsp::coef`): `env += (|x| - env) * (attack or release)`.
pub struct Follower {
    pub env: f32,
    c_att: f32,
    c_rel: f32,
}

impl Follower {
    pub const NEW: Self = Self { env: 0.0, c_att: 1.0, c_rel: 1.0 };

    /// Attack and release time constants (s).
    pub fn times(&mut self, attack: f32, release: f32) {
        self.c_att = coef(attack);
        self.c_rel = coef(release);
    }

    /// Follows `a` (already rectified, e.g. `x.abs()`); returns the envelope.
    #[inline]
    pub fn follow(&mut self, a: f32) -> f32 {
        self.env += (a - self.env) * if a > self.env { self.c_att } else { self.c_rel };
        self.env
    }
}

/// Pick-attack detector: a slow copy of a fast envelope; an onset is the fast
/// one jumping well above the slow one.
pub struct Onset {
    pub slow: f32,
    c_slow: f32,
}

impl Onset {
    pub const NEW: Self = Self { slow: 0.0, c_slow: 1.0 };

    /// The slow envelope's time constant (s).
    pub fn time(&mut self, slow: f32) {
        self.c_slow = coef(slow);
    }

    /// Tracks the fast envelope `env`.
    #[inline]
    pub fn update(&mut self, env: f32) {
        self.slow += (env - self.slow) * self.c_slow;
    }

    /// The fast envelope is `ratio` times the slow one and above `min`.
    #[inline]
    pub fn hit(&self, env: f32, ratio: f32, min: f32) -> bool {
        env > ratio * self.slow && env > min
    }
}

/// Peak follower in the "multiply by e^(-1/(t*fs))" form, with times in ms
/// and any sample rate (as in the C++ SDK pedals).
pub struct ExpEnvelope {
    pub env: f32,
    att: f32,
    rel: f32,
}

impl ExpEnvelope {
    pub const NEW: Self = Self { env: 0.0, att: 0.0, rel: 0.0 };

    pub fn set(&mut self, att_ms: f32, rel_ms: f32, fs: f32) {
        self.att = exp_hq(-1.0 / (att_ms * 0.001 * fs));
        self.rel = exp_hq(-1.0 / (rel_ms * 0.001 * fs));
    }

    pub fn reset(&mut self) {
        self.env = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let a = x.abs();
        let c = if a > self.env { self.att } else { self.rel };
        self.env = c * self.env + (1.0 - c) * a;
        self.env
    }
}
