//! Mosrite Fuzzrite-style fuzz: two clipping transistor stages, blended out
//! of phase by Depth.

use super::presets::{STAGE2_MIN, STAGE2_RANGE};
use crate::pedalkit::dsp::{Biquad, tanh};
use crate::pedalkit::helpers::Clip2x;

pub struct Fuzzrite {
    hp: Biquad,
    lp: Biquad,
    nasal: Biquad,
    clip: Clip2x,
}

impl Fuzzrite {
    pub const NEW: Self = Self { hp: Biquad::NEW, lp: Biquad::NEW, nasal: Biquad::NEW, clip: Clip2x::NEW };

    pub fn init(&mut self) {
        self.hp.highpass(160.0, 0.707); // Fuzzrites are thin
        self.lp.lowpass(6500.0, 0.707);
        self.nasal.peak(1800.0, 1.2, 4.0);
    }

    /// `g` fuzz gain, `depth` the blend, `mk` makeup.
    #[inline]
    pub fn process(&mut self, x: f32, g: f32, depth: f32, mk: f32) -> f32 {
        let u = self.hp.process(x) * g;
        let a = self.clip.process(u, stage1);
        let b = (a * (STAGE2_MIN + STAGE2_RANGE * depth)).clamp(-1.0, 1.0);
        let y = b * depth - 0.5 * a * (1.0 - depth);
        self.nasal.process(self.lp.process(y)) * mk
    }
}

/// First Fuzzrite transistor: starved and asymmetric (soft on top, hard on the bottom).
#[inline]
fn stage1(u: f32) -> f32 {
    if u > 0.0 { tanh(u) } else { (u * 1.4).max(-0.7) }
}
