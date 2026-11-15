//! Organ double: squares at the tracked pitch and an octave below (a
//! Schmitt trigger on the low-passed guitar, flipping a divider), shaped by
//! the guitar's envelope, with a combo-organ amplitude vibrato.

use super::presets::*;
use crate::pedalkit::dsp::{Biquad, SR, sin};
use crate::pedalkit::env::Follower;
use crate::pedalkit::helpers::Phase;
use core::f32::consts::TAU;

pub struct Organ {
    trk: [Biquad; 2],
    trk_env: Follower,
    high: bool,
    sub: bool,
    env: Follower,
    lp: Biquad,
    hp: Biquad,
    vib: Phase,
}

impl Organ {
    pub const NEW: Self = Self {
        trk: [Biquad::NEW; 2],
        trk_env: Follower::NEW,
        high: false,
        sub: false,
        env: Follower::NEW,
        lp: Biquad::NEW,
        hp: Biquad::NEW,
        vib: Phase::new(0.0),
    };

    pub fn init(&mut self) {
        self.trk[0].lowpass(TRACK_HZ, 0.707);
        self.trk[1].lowpass(TRACK_HZ, 0.707);
        self.trk_env.times(0.001, 0.08);
        self.env.times(0.004, 0.12);
        self.lp.lowpass(2500.0, 0.707);
        self.hp.highpass(30.0, 0.707);
    }

    /// `x` the guitar, `in_env` its envelope.
    #[inline]
    pub fn process(&mut self, x: f32, in_env: f32) -> f32 {
        let t = self.trk[0].process(x);
        let t = self.trk[1].process(t);
        let th = TRACK_THRESH * self.trk_env.follow(t.abs());
        if !self.high && t > th {
            self.high = true;
            self.sub = !self.sub;
        } else if self.high && t < -th {
            self.high = false;
        }
        let fund = if self.high { 1.0 } else { -1.0 };
        let sub = if self.sub { 1.0 } else { -1.0 };
        let env = self.env.follow(in_env);
        // Combo-organ wobble, as a gentle amplitude vibrato.
        let vib = 1.0 + ORGAN_VIB_DEPTH * sin(TAU * self.vib.step(ORGAN_VIB_HZ / SR));
        let o = (ORGAN_SUB * sub + ORGAN_FUND * fund) * env * ORGAN_GAIN * vib;
        self.hp.process(self.lp.process(o))
    }
}
