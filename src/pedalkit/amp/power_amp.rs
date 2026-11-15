//! Power amp: the supply sags on hard hits (compression and bloom), the
//! output valves clip, then the resonance restores the low end after the
//! clipping, where it can't turn to mud.

use super::dsp::{Biquad, coef, db, tanh};
use super::helpers::Clip2x;
use super::voicing::Voicing;

pub struct PowerAmp {
    drive: f32,
    bias: f32,
    bias_off: f32,
    sag_k: f32,
    env: f32,
    c_att: f32,
    c_rel: f32,
    out: f32,
    clip: Clip2x,
    reso: Biquad,
}

impl PowerAmp {
    pub const NEW: Self = Self {
        drive: 1.0,
        bias: 0.0,
        bias_off: 0.0,
        sag_k: 0.0,
        env: 0.0,
        c_att: 1.0,
        c_rel: 1.0,
        out: 1.0,
        clip: Clip2x::NEW,
        reso: Biquad::NEW,
    };

    pub fn init(&mut self, v: &Voicing) {
        self.drive = db(v.power_db);
        self.bias = v.power_bias;
        self.bias_off = tanh(v.power_bias);
        self.sag_k = v.sag;
        self.c_att = coef(v.sag_attack);
        self.c_rel = coef(v.sag_release);
        self.out = v.out;
        v.reso.apply(&mut self.reso);
    }

    #[inline]
    pub fn process(&mut self, y: f32) -> f32 {
        let a = y.abs();
        self.env += (a - self.env) * if a > self.env { self.c_att } else { self.c_rel };
        let u = y * self.drive / (1.0 + self.sag_k * self.env);
        let (b, off) = (self.bias, self.bias_off);
        let y = self.clip.process(u, |u| tanh(u + b) - off);
        self.reso.process(y) * self.out
    }
}
