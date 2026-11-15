//! A wah: a resonant band-pass peak added in parallel and faded in, so
//! switching never clicks. Park it (`init` sets `park_hz`) or sweep it from a
//! knob, envelope or LFO with `set_freq` / `set_position`.

use super::dsp::{Biquad, db, exp2, log2};
use super::helpers::Smooth;

pub struct WahSettings {
    /// Where the wah sits after `init` (Hz).
    pub park_hz: f32,
    /// Sweep range for `set_position` (Hz, heel to toe).
    pub heel_hz: f32,
    pub toe_hz: f32,
    /// Resonance and peak gain (dB).
    pub q: f32,
    pub boost_db: f32,
    /// Fade time (s).
    pub fade: f32,
}

pub struct Wah {
    amount: Smooth,
    k: f32,
    q: f32,
    heel: f32,
    /// log2(toe / heel): the sweep's octaves.
    octaves: f32,
    bp: Biquad,
}

impl Wah {
    pub const NEW: Self = Self { amount: Smooth::new(0.0), k: 0.0, q: 1.0, heel: 400.0, octaves: 0.0, bp: Biquad::NEW };

    pub fn init(&mut self, s: &WahSettings) {
        self.amount.time(s.fade);
        self.k = db(s.boost_db) - 1.0;
        self.q = s.q;
        self.heel = s.heel_hz;
        self.octaves = log2(s.toe_hz / s.heel_hz);
        self.set_freq(s.park_hz);
    }

    pub fn on(&mut self, on: bool) {
        self.amount.on(on);
    }

    pub fn snap(&mut self) {
        self.amount.snap();
    }

    /// Moves the peak to `hz`.
    pub fn set_freq(&mut self, hz: f32) {
        self.bp.bandpass(hz, self.q);
    }

    /// Moves the peak along the sweep: 0 = heel, 1 = toe (exponential, like a pedal).
    pub fn set_position(&mut self, p: f32) {
        self.set_freq(self.heel * exp2(p.clamp(0.0, 1.0) * self.octaves));
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let a = self.amount.tick();
        x + a * self.k * self.bp.process(x)
    }
}
