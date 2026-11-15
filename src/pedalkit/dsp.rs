//! Small `no_std` DSP toolkit. Everything is f32 (the Endless FPU is single
//! precision only) and there is no libm, so the math functions here are local
//! approximations, accurate enough for audio.

use core::f32::consts::{FRAC_PI_2, LN_2, LOG2_E, PI, TAU};

pub const SR: f32 = 48_000.0;

#[inline]
pub fn floor(x: f32) -> f32 {
    let i = x as i32 as f32;
    if i > x { i - 1.0 } else { i }
}

/// sin(x) for any moderate x; |error| < 1e-6.
pub fn sin(x: f32) -> f32 {
    let mut r = x - TAU * floor(x * (1.0 / TAU) + 0.5); // [-pi, pi)
    if r > FRAC_PI_2 {
        r = PI - r;
    } else if r < -FRAC_PI_2 {
        r = -PI - r;
    }
    let r2 = r * r;
    r * (1.0
        + r2 * (-1.666_666_7e-1
            + r2 * (8.333_333e-3 + r2 * (-1.984_127e-4 + r2 * (2.755_732e-6 + r2 * -2.505_211e-8)))))
}

/// Square root (bit-trick start, three Newton steps; relative error < 1e-7).
pub fn sqrt(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut y = f32::from_bits((x.to_bits() >> 1) + 0x1fbd_1df5);
    y = 0.5 * (y + x / y);
    y = 0.5 * (y + x / y);
    0.5 * (y + x / y)
}

/// x^y for x >= 0 (0^y = 0).
pub fn powf(x: f32, y: f32) -> f32 {
    if x <= 0.0 { 0.0 } else { exp2_hq(y * log2(x)) }
}

pub fn cos(x: f32) -> f32 {
    sin(x + FRAC_PI_2)
}

/// 2^x; relative error < 2e-5.
pub fn exp2(x: f32) -> f32 {
    let x = x.clamp(-126.0, 126.0);
    let i = floor(x);
    let t = (x - i) * LN_2;
    let p = 1.0 + t * (1.0 + t * (0.5 + t * (1.0 / 6.0 + t * (1.0 / 24.0 + t * (1.0 / 120.0 + t * (1.0 / 720.0))))));
    p * f32::from_bits(((i as i32 + 127) as u32) << 23)
}

/// log2(x) for x > 0; |error| < 1e-5.
pub fn log2(x: f32) -> f32 {
    let bits = x.to_bits();
    let e = ((bits >> 23) & 0xff) as i32 - 127;
    let m = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000); // [1, 2)
    // ln(m) = 2 atanh(s), s = (m - 1) / (m + 1) <= 1/3.
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    e as f32 + 2.0 * LOG2_E * s * (1.0 + s2 * (1.0 / 3.0 + s2 * (1.0 / 5.0 + s2 * (1.0 / 7.0 + s2 * (1.0 / 9.0)))))
}

pub fn exp(x: f32) -> f32 {
    exp2(x * LOG2_E)
}

/// 2^x to near f32 precision (relative error < 1e-7): round-to-nearest
/// range reduction and a degree-7 polynomial. For coefficients that sit
/// close to 1 (decays like e^(-1/(t*sr))), where `exp2`'s 2e-5 would be a
/// large error in the time constant. Used by the blocks ported from the C++
/// SDK pedals; `exp2` stays as it is so the other pedals sound the same.
pub fn exp2_hq(x: f32) -> f32 {
    let x = x.clamp(-126.0, 126.0);
    let i = floor(x + 0.5);
    let t = (x - i) * LN_2;
    let p = 1.0
        + t * (1.0
            + t * (0.5 + t * (1.0 / 6.0 + t * (1.0 / 24.0 + t * (1.0 / 120.0 + t * (1.0 / 720.0 + t * (1.0 / 5040.0)))))));
    p * f32::from_bits(((i as i32 + 127) as u32) << 23)
}

/// e^x to near f32 precision; see `exp2_hq`.
pub fn exp_hq(x: f32) -> f32 {
    exp2_hq(x * LOG2_E)
}

/// Decibels to linear gain.
pub fn db(db: f32) -> f32 {
    exp2(db * 0.166_096_4) // log2(10) / 20
}

/// Smooth saturator: Padé tanh, exactly ±1 (with zero slope) beyond |x| = 3.
#[inline]
pub fn tanh(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    let x2 = x * x;
    x * (27.0 + x2) / (27.0 + 9.0 * x2)
}

/// One-pole smoothing coefficient for time constant `tau` seconds.
pub fn coef(tau: f32) -> f32 {
    1.0 - exp(-1.0 / (tau * SR))
}

/// One-pole low-pass.
#[derive(Clone, Copy)]
pub struct OnePole {
    a: f32,
    z: f32,
}

impl OnePole {
    pub const NEW: Self = Self { a: 1.0, z: 0.0 };
    pub fn set(&mut self, fc: f32) {
        self.a = 1.0 - exp(-TAU * fc / SR);
    }
    /// The same at any sample rate.
    pub fn set_at(&mut self, fc: f32, sr: f32) {
        self.a = 1.0 - exp_hq(-TAU * fc / sr);
    }
    pub fn reset(&mut self) {
        self.z = 0.0;
    }
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.z += self.a * (x - self.z);
        self.z
    }
}

/// One-pole high-pass (a coupling cap / DC block), at any sample rate:
/// `y = a·(y1 + x - x1)`, `a = e^(-2π·f/sr)`.
#[derive(Clone, Copy)]
pub struct OnePoleHp {
    a: f32,
    x1: f32,
    y1: f32,
}

impl OnePoleHp {
    pub const NEW: Self = Self { a: 0.0, x1: 0.0, y1: 0.0 };
    pub fn set_at(&mut self, fc: f32, sr: f32) {
        self.a = exp_hq(-TAU * fc / sr);
    }
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.a * (self.y1 + x - self.x1);
        self.x1 = x;
        self.y1 = y;
        y
    }
}

/// RBJ-cookbook biquad, transposed direct form II.
#[derive(Clone, Copy)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub const NEW: Self = Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, z1: 0.0, z2: 0.0 };

    fn set(&mut self, b: [f32; 3], a: [f32; 3]) {
        let n = 1.0 / a[0];
        self.b0 = b[0] * n;
        self.b1 = b[1] * n;
        self.b2 = b[2] * n;
        self.a1 = a[1] * n;
        self.a2 = a[2] * n;
    }

    fn w(f: f32, q: f32) -> (f32, f32) {
        let w = TAU * f / SR;
        (cos(w), sin(w) / (2.0 * q))
    }

    pub fn lowpass(&mut self, f: f32, q: f32) -> &mut Self {
        let (c, al) = Self::w(f, q);
        self.set([(1.0 - c) * 0.5, 1.0 - c, (1.0 - c) * 0.5], [1.0 + al, -2.0 * c, 1.0 - al]);
        self
    }

    pub fn highpass(&mut self, f: f32, q: f32) -> &mut Self {
        let (c, al) = Self::w(f, q);
        self.set([(1.0 + c) * 0.5, -(1.0 + c), (1.0 + c) * 0.5], [1.0 + al, -2.0 * c, 1.0 - al]);
        self
    }

    /// Band-pass with 0 dB peak gain.
    pub fn bandpass(&mut self, f: f32, q: f32) -> &mut Self {
        let (c, al) = Self::w(f, q);
        self.set([al, 0.0, -al], [1.0 + al, -2.0 * c, 1.0 - al]);
        self
    }

    pub fn peak(&mut self, f: f32, q: f32, gain_db: f32) -> &mut Self {
        let (c, al) = Self::w(f, q);
        let a = db(gain_db * 0.5);
        self.set([1.0 + al * a, -2.0 * c, 1.0 - al * a], [1.0 + al / a, -2.0 * c, 1.0 - al / a]);
        self
    }

    // The same filters at any sample rate (e.g. inside a 2x-oversampled
    // stage), plus shelves (slope 1), in the Audio EQ Cookbook form.

    fn w_at(f: f32, q: f32, sr: f32) -> (f32, f32, f32) {
        let w = TAU * f / sr;
        (cos(w), sin(w), sin(w) / (2.0 * q))
    }

    pub fn lowpass_at(&mut self, f: f32, q: f32, sr: f32) -> &mut Self {
        let (c, _, al) = Self::w_at(f, q, sr);
        self.set([(1.0 - c) * 0.5, 1.0 - c, (1.0 - c) * 0.5], [1.0 + al, -2.0 * c, 1.0 - al]);
        self
    }

    pub fn highpass_at(&mut self, f: f32, q: f32, sr: f32) -> &mut Self {
        let (c, _, al) = Self::w_at(f, q, sr);
        self.set([(1.0 + c) * 0.5, -(1.0 + c), (1.0 + c) * 0.5], [1.0 + al, -2.0 * c, 1.0 - al]);
        self
    }

    pub fn peak_at(&mut self, f: f32, q: f32, gain_db: f32, sr: f32) -> &mut Self {
        let (c, _, al) = Self::w_at(f, q, sr);
        let a = db(gain_db * 0.5);
        self.set([1.0 + al * a, -2.0 * c, 1.0 - al * a], [1.0 + al / a, -2.0 * c, 1.0 - al / a]);
        self
    }

    pub fn low_shelf_at(&mut self, f: f32, gain_db: f32, sr: f32) -> &mut Self {
        let a = db(gain_db * 0.5);
        let (c, s, _) = Self::w_at(f, 1.0, sr);
        let al = s * 0.5 * core::f32::consts::SQRT_2;
        let sq = 2.0 * sqrt(a) * al;
        self.set(
            [a * ((a + 1.0) - (a - 1.0) * c + sq), 2.0 * a * ((a - 1.0) - (a + 1.0) * c), a * ((a + 1.0) - (a - 1.0) * c - sq)],
            [(a + 1.0) + (a - 1.0) * c + sq, -2.0 * ((a - 1.0) + (a + 1.0) * c), (a + 1.0) + (a - 1.0) * c - sq],
        );
        self
    }

    pub fn high_shelf_at(&mut self, f: f32, gain_db: f32, sr: f32) -> &mut Self {
        let a = db(gain_db * 0.5);
        let (c, s, _) = Self::w_at(f, 1.0, sr);
        let al = s * 0.5 * core::f32::consts::SQRT_2;
        let sq = 2.0 * sqrt(a) * al;
        self.set(
            [a * ((a + 1.0) + (a - 1.0) * c + sq), -2.0 * a * ((a - 1.0) + (a + 1.0) * c), a * ((a + 1.0) + (a - 1.0) * c - sq)],
            [(a + 1.0) - (a - 1.0) * c + sq, 2.0 * ((a - 1.0) - (a + 1.0) * c), (a + 1.0) - (a - 1.0) * c - sq],
        );
        self
    }

    /// Clears the filter's memory.
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// Delay line of N samples (N a power of two) with fractional reads.
pub struct Delay<const N: usize> {
    buf: [f32; N],
    w: usize,
}

impl<const N: usize> Delay<N> {
    pub const NEW: Self = Self { buf: [0.0; N], w: 0 };

    #[inline]
    pub fn push(&mut self, x: f32) {
        self.w = (self.w + 1) & (N - 1);
        self.buf[self.w] = x;
    }

    /// The sample `d` samples ago (linear interpolation; d = 0 is the newest).
    #[inline]
    pub fn read(&self, d: f32) -> f32 {
        let d = d.clamp(0.0, (N - 2) as f32);
        let i = d as usize;
        let fr = d - i as f32;
        let a = self.buf[self.w.wrapping_sub(i) & (N - 1)];
        let b = self.buf[self.w.wrapping_sub(i + 1) & (N - 1)];
        a + (b - a) * fr
    }
}
