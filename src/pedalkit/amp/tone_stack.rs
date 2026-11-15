//! Tone stack: the voicing's bass, mid and upper bands, then a treble control
//! on the content above the voicing's treble split.

use super::dsp::{Biquad, OnePole};
use super::voicing::Voicing;

pub struct ToneStack {
    bass: Biquad,
    mid: Biquad,
    upper: Biquad,
    tre_split: OnePole,
    tre_min: f32,
    tre_span: f32,
}

impl ToneStack {
    pub const NEW: Self = Self {
        bass: Biquad::NEW,
        mid: Biquad::NEW,
        upper: Biquad::NEW,
        tre_split: OnePole::NEW,
        tre_min: 1.0,
        tre_span: 0.0,
    };

    pub fn init(&mut self, v: &Voicing) {
        v.bass.apply(&mut self.bass);
        v.mid.apply(&mut self.mid);
        v.upper.apply(&mut self.upper);
        self.tre_split.set(v.treble_hz);
        self.tre_min = v.treble_min;
        self.tre_span = v.treble_span;
    }

    /// `tone` 0..1: the treble control.
    #[inline]
    pub fn process(&mut self, y: f32, tone: f32) -> f32 {
        let y = self.upper.process(self.mid.process(self.bass.process(y)));
        let lo = self.tre_split.process(y);
        lo + (self.tre_min + self.tre_span * tone) * (y - lo)
    }
}
