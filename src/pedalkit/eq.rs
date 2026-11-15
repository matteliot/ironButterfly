//! A fixed chain of biquads set from a list: cabinets, amp voicings, any EQ.

use super::dsp::Biquad;

#[derive(Clone, Copy)]
pub enum Eq {
    /// High-pass: corner (Hz), Q.
    Hp(f32, f32),
    /// Low-pass: corner (Hz), Q.
    Lp(f32, f32),
    /// Peak: centre (Hz), Q, gain (dB).
    Peak(f32, f32, f32),
    /// Band-pass (0 dB peak): centre (Hz), Q.
    Bp(f32, f32),
}

impl Eq {
    pub fn apply(&self, b: &mut Biquad) {
        match *self {
            Eq::Hp(f, q) => b.highpass(f, q),
            Eq::Lp(f, q) => b.lowpass(f, q),
            Eq::Peak(f, q, g) => b.peak(f, q, g),
            Eq::Bp(f, q) => b.bandpass(f, q),
        };
    }
}

/// `N` biquads in series.
pub struct EqChain<const N: usize> {
    b: [Biquad; N],
}

impl<const N: usize> EqChain<N> {
    pub const NEW: Self = Self { b: [Biquad::NEW; N] };

    pub fn init(&mut self, stages: &[Eq; N]) {
        for (b, s) in self.b.iter_mut().zip(stages.iter()) {
            s.apply(b);
        }
    }

    /// Clears the filters' memory (keeps their settings).
    pub fn reset(&mut self) {
        for b in self.b.iter_mut() {
            b.reset();
        }
    }

    #[inline]
    pub fn process(&mut self, mut x: f32) -> f32 {
        for b in self.b.iter_mut() {
            x = b.process(x);
        }
        x
    }
}
