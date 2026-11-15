//! Noise gate: an envelope with hysteresis (opens above one level, closes
//! below a lower one) and a smoothed gain, so it never chatters or clicks.

use super::dsp::coef;
use super::env::Follower;

pub struct GateSettings {
    /// Open and close thresholds on the envelope (linear).
    pub open: f32,
    pub close: f32,
    /// Envelope attack and release (s).
    pub attack: f32,
    pub release: f32,
    /// Gain fade-in and fade-out (s).
    pub fade_in: f32,
    pub fade_out: f32,
}

pub struct Gate {
    pub env: Follower,
    pub open: bool,
    pub gain: f32,
    open_at: f32,
    close_at: f32,
    c_in: f32,
    c_out: f32,
}

impl Gate {
    pub const NEW: Self =
        Self { env: Follower::NEW, open: false, gain: 0.0, open_at: 0.0, close_at: 0.0, c_in: 1.0, c_out: 1.0 };

    pub fn init(&mut self, s: &GateSettings) {
        self.env.times(s.attack, s.release);
        self.open_at = s.open;
        self.close_at = s.close;
        self.c_in = coef(s.fade_in);
        self.c_out = coef(s.fade_out);
    }

    /// The fade-out coefficient (for `process_with`).
    pub fn c_out(&self) -> f32 {
        self.c_out
    }

    /// Follows `x`; returns the gate's gain (0..1).
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.process_with(x, 1.0, self.c_out)
    }

    /// As `process`, with the thresholds scaled by `scale` and fade-out coefficient `c_out`.
    #[inline]
    pub fn process_with(&mut self, x: f32, scale: f32, c_out: f32) -> f32 {
        let env = self.env.follow(x.abs());
        if env > self.open_at * scale {
            self.open = true;
        } else if env < self.close_at * scale {
            self.open = false;
        }
        let (gt, gc) = if self.open { (1.0, self.c_in) } else { (0.0, c_out) };
        self.gain += (gt - self.gain) * gc;
        self.gain
    }
}
