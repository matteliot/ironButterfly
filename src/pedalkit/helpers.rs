//! Small building blocks shared by more than one part of the patch.

use super::dsp::{Biquad, coef};

/// A value that glides toward a target with a one-pole time constant: knobs
/// (so they never zipper) and switched effects (so they crossfade, never click).
pub struct Smooth {
    v: f32,
    t: f32,
    c: f32,
}

impl Smooth {
    pub const fn new(v: f32) -> Self {
        Self { v, t: v, c: 1.0 }
    }

    /// The glide time constant (s).
    pub fn time(&mut self, tau: f32) {
        self.c = coef(tau);
    }

    pub fn set(&mut self, t: f32) {
        self.t = t;
    }

    /// Target 1 when on, 0 when off.
    pub fn on(&mut self, on: bool) {
        self.t = if on { 1.0 } else { 0.0 };
    }

    /// Jump straight to the target.
    pub fn snap(&mut self) {
        self.v = self.t;
    }

    /// The current value.
    pub fn value(&self) -> f32 {
        self.v
    }

    /// One sample of glide; returns the new value.
    #[inline]
    pub fn tick(&mut self) -> f32 {
        self.v += (self.t - self.v) * self.c;
        self.v
    }
}

/// One EQ band: centre (Hz), Q and gain (dB).
#[derive(Clone, Copy)]
pub struct Peak {
    pub f: f32,
    pub q: f32,
    pub db: f32,
}

impl Peak {
    pub const fn new(f: f32, q: f32, db: f32) -> Self {
        Self { f, q, db }
    }

    pub fn apply(&self, b: &mut Biquad) {
        b.peak(self.f, self.q, self.db);
    }
}

/// A clipper run at 2x: the half-sample midpoint is clipped too and the two
/// results averaged, which tames aliasing from the hard-driven stages.
pub struct Clip2x {
    prev: f32,
}

impl Clip2x {
    pub const NEW: Self = Self { prev: 0.0 };

    #[inline]
    pub fn process(&mut self, u: f32, f: impl Fn(f32) -> f32) -> f32 {
        let y = 0.5 * (f(0.5 * (u + self.prev)) + f(u));
        self.prev = u;
        y
    }
}

/// Output soft limiter, never reaching ±1.
#[inline]
pub fn limit(x: f32) -> f32 {
    super::dsp::tanh(x) * 0.98
}

/// A phase accumulator in 0..1.
pub struct Phase {
    pub p: f32,
}

impl Phase {
    pub const fn new(p: f32) -> Self {
        Self { p }
    }

    /// Advances by `inc` (cycles), wrapping once; returns the new phase.
    #[inline]
    pub fn step(&mut self, inc: f32) -> f32 {
        self.p += inc;
        if self.p >= 1.0 {
            self.p -= 1.0;
        }
        self.p
    }
}

/// Hard clipper: cubic knee, flat beyond |u| = 1.5.
#[inline]
pub fn hard(u: f32) -> f32 {
    let c = (u * (1.0 / 1.5)).clamp(-1.0, 1.0);
    1.5 * c - 0.5 * c * c * c
}

/// Biased tube-style saturation, zero at rest: `tanh(u + bias) - tanh(bias)`.
#[inline]
pub fn biased(u: f32, bias: f32) -> f32 {
    super::dsp::tanh(u + bias) - super::dsp::tanh(bias)
}

/// Soft-knee limiter: transparent below `knee`, approaching ±1 above it.
#[inline]
pub fn soft_limit(x: f32, knee: f32) -> f32 {
    let a = x.abs();
    if a <= knee {
        return x;
    }
    let over = (a - knee) / (1.0 - knee);
    let y = knee + (1.0 - knee) * over / (1.0 + over);
    if x < 0.0 { -y } else { y }
}
