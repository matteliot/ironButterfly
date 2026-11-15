//! Amp feedback: the output, band-passed, comes back into the input after a
//! short delay (the distance from the amp to the guitar), faded up slowly
//! while a note sustains and quickly down when it stops.

use super::dsp::{Biquad, coef};
use super::pool::{Line, Pool};

pub struct FeedbackSettings {
    /// Loop delay (samples, < 512).
    pub delay: f32,
    /// Band-pass centre (Hz) and Q: where the howl blooms.
    pub freq: f32,
    pub q: f32,
    /// Loop gain at full bloom.
    pub gain: f32,
    /// Bloom and die time constants (s).
    pub bloom: f32,
    pub die: f32,
}

pub struct Feedback {
    line: Line,
    bp: Biquad,
    pub ramp: f32,
    delay: f32,
    gain: f32,
    q: f32,
    c_up: f32,
    c_down: f32,
}

impl Feedback {
    pub const NEW: Self =
        Self { line: Line::NEW, bp: Biquad::NEW, ramp: 0.0, delay: 0.0, gain: 0.0, q: 1.0, c_up: 1.0, c_down: 1.0 };

    pub fn init(&mut self, s: &FeedbackSettings, pool: &mut Pool) {
        self.line.take(pool, 512);
        self.bp.bandpass(s.freq, s.q);
        self.delay = s.delay;
        self.gain = s.gain;
        self.q = s.q;
        self.c_up = coef(s.bloom);
        self.c_down = coef(s.die);
    }

    /// Moves the howl's centre (Hz).
    pub fn tune(&mut self, hz: f32) {
        self.bp.bandpass(hz, self.q);
    }

    /// Fades the loop towards on (`true`) or off.
    #[inline]
    pub fn drive(&mut self, on: bool) {
        let t = if on { 1.0 } else { 0.0 };
        self.ramp += (t - self.ramp) * if t > self.ramp { self.c_up } else { self.c_down };
    }

    /// What comes back into the input, divided by `norm` (the gain it will
    /// meet, so the bloom is the same at any setting).
    #[inline]
    pub fn read(&self, norm: f32) -> f32 {
        self.line.read(self.delay) * self.ramp * self.gain / norm
    }

    /// Sends the output `y` into the loop.
    #[inline]
    pub fn send(&mut self, y: f32) {
        let b = self.bp.process(y);
        self.line.push(b);
    }
}
