//! A valve head: preamp -> tone stack -> sagging power amp, all set by a `Voicing`.

// The stages reach the kit through these.
use super::{dsp, helpers};

mod power_amp;
mod preamp;
mod tone_stack;
pub mod voicing;

use power_amp::PowerAmp;
use preamp::Preamp;
use tone_stack::ToneStack;
use voicing::Voicing;

pub struct Amp {
    pre: Preamp,
    tone: ToneStack,
    power: PowerAmp,
}

impl Amp {
    pub const NEW: Self = Self { pre: Preamp::NEW, tone: ToneStack::NEW, power: PowerAmp::NEW };

    pub fn init(&mut self, v: &Voicing) {
        self.pre.init(v);
        self.tone.init(v);
        self.power.init(v);
    }

    /// `g` preamp gain (linear), `tone` 0..1 (the treble control).
    #[inline]
    pub fn process(&mut self, x: f32, g: f32, tone: f32) -> f32 {
        let y = self.pre.process(x, g);
        let y = self.tone.process(y, tone);
        self.power.process(y)
    }
}
