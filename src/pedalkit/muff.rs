//! Big Muff-style fuzz: two soft clippers with low-passes between, then the
//! Muff tone blend (a low-pass against a high-pass, the mid scoop). It always
//! runs and is crossfaded in, so switching never clicks.

use super::dsp::{Biquad, OnePole, db, tanh};
use super::helpers::Smooth;

pub struct MuffSettings {
    /// Input high-pass (Hz).
    pub hp_hz: f32,
    /// Gain into the first clipper (dB).
    pub gain_db: f32,
    /// Low-pass after the first clipper (Hz).
    pub lp1_hz: f32,
    /// Gain into the second clipper (linear).
    pub stage2: f32,
    /// Low-pass after the second clipper (Hz).
    pub lp2_hz: f32,
    /// Tone blend: low-pass (Hz) and its mix, high-pass (Hz) and its mix.
    pub tone_lo_hz: f32,
    pub tone_lo_mix: f32,
    pub tone_hi_hz: f32,
    pub tone_hi_mix: f32,
    /// Output trim (linear).
    pub out: f32,
    /// Crossfade time (s).
    pub fade: f32,
}

pub struct Muff {
    mix: Smooth,
    gain: f32,
    stage2: f32,
    lo_mix: f32,
    hi_mix: f32,
    out: f32,
    hp: Biquad,
    lp1: OnePole,
    lp2: OnePole,
    lo: OnePole,
    hi: OnePole,
}

impl Muff {
    pub const NEW: Self = Self {
        mix: Smooth::new(0.0),
        gain: 1.0,
        stage2: 1.0,
        lo_mix: 0.0,
        hi_mix: 0.0,
        out: 0.0,
        hp: Biquad::NEW,
        lp1: OnePole::NEW,
        lp2: OnePole::NEW,
        lo: OnePole::NEW,
        hi: OnePole::NEW,
    };

    pub fn init(&mut self, s: &MuffSettings) {
        self.mix.time(s.fade);
        self.gain = db(s.gain_db);
        self.stage2 = s.stage2;
        self.lo_mix = s.tone_lo_mix;
        self.hi_mix = s.tone_hi_mix;
        self.out = s.out;
        self.hp.highpass(s.hp_hz, 0.707);
        self.lp1.set(s.lp1_hz);
        self.lp2.set(s.lp2_hz);
        self.lo.set(s.tone_lo_hz);
        self.hi.set(s.tone_hi_hz);
    }

    pub fn on(&mut self, on: bool) {
        self.mix.on(on);
    }

    pub fn snap(&mut self) {
        self.mix.snap();
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let m = self.mix.tick();
        let y = tanh(self.hp.process(x) * self.gain);
        let y = self.lp1.process(y);
        let y = tanh(y * self.stage2);
        let y = self.lp2.process(y);
        let lo = self.lo.process(y);
        let hi = y - self.hi.process(y);
        let f = (self.lo_mix * lo + self.hi_mix * hi) * self.out;
        x + (f - x) * m
    }
}
