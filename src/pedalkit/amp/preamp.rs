//! Preamp: a biased triode, the interstage filters, then a second gain stage.

use super::dsp::{Biquad, db, tanh};
use super::helpers::Clip2x;
use super::voicing::Voicing;

pub struct Preamp {
    trim: f32,
    bias: f32,
    bias_off: f32,
    stage2: f32,
    hp_in: Biquad,
    triode: Clip2x,
    dc: Biquad,
    inter: Biquad,
    clip2: Clip2x,
}

impl Preamp {
    pub const NEW: Self = Self {
        trim: 1.0,
        bias: 0.0,
        bias_off: 0.0,
        stage2: 1.0,
        hp_in: Biquad::NEW,
        triode: Clip2x::NEW,
        dc: Biquad::NEW,
        inter: Biquad::NEW,
        clip2: Clip2x::NEW,
    };

    pub fn init(&mut self, v: &Voicing) {
        self.trim = db(v.trim_db);
        self.bias = v.bias;
        self.bias_off = tanh(v.bias);
        self.stage2 = db(v.stage2_db);
        self.hp_in.highpass(v.hp_in_hz, 0.707);
        self.dc.highpass(v.dc_hz, 0.707);
        self.inter.lowpass(v.inter_lp_hz, 0.707);
    }

    /// `g` preamp gain (linear).
    #[inline]
    pub fn process(&mut self, x: f32, g: f32) -> f32 {
        let u = self.hp_in.process(x) * g * self.trim;
        let (b, off) = (self.bias, self.bias_off);
        let y = self.triode.process(u, |u| tanh(u + b) - off);
        let y = self.inter.process(self.dc.process(y));
        self.clip2.process(y * self.stage2, tanh)
    }
}
