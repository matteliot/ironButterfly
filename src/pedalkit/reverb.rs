//! Freeverb-style stereo reverb: parallel low-pass-feedback combs into
//! series Schroeder allpasses per side, the right side's lengths spread.
//! Buffers come from the pool, sized exactly.

use super::pool::{Line, Pool, Ring};

pub struct ReverbSettings {
    /// Comb and allpass lengths for the left side (samples, up to 8 and 4).
    pub combs: &'static [usize],
    pub allpasses: &'static [usize],
    /// Added to every length on the right side.
    pub spread: usize,
    /// Comb damping (the low-pass in the loop).
    pub damp: f32,
    /// Input gain into the combs, and output gain.
    pub in_gain: f32,
    pub out_gain: f32,
    /// Pre-delay (samples, < 2048), or `None` for none.
    pub predelay: Option<f32>,
}

/// Freeverb's tunings (44.1 kHz) scaled to 48 kHz.
pub const FREEVERB_COMBS: [usize; 8] = [1214, 1293, 1389, 1475, 1547, 1622, 1694, 1759];
pub const FREEVERB_ALLPASSES: [usize; 4] = [605, 480, 371, 245];

/// Low-pass-feedback comb.
struct Comb {
    r: Ring,
    filt: f32,
}

impl Comb {
    const NEW: Self = Self { r: Ring::NEW, filt: 0.0 };

    #[inline]
    fn process(&mut self, x: f32, fb: f32, damp: f32) -> f32 {
        let o = self.r.get();
        self.filt = o + (self.filt - o) * damp;
        self.r.set(x + self.filt * fb);
        self.r.step();
        o
    }
}

/// Schroeder allpass, feedback 0.5.
struct Allpass {
    r: Ring,
}

impl Allpass {
    const NEW: Self = Self { r: Ring::NEW };

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let o = self.r.get();
        self.r.set(x + o * 0.5);
        self.r.step();
        o - x
    }
}

pub struct Reverb {
    pre: Line,
    predelay: Option<f32>,
    cl: [Comb; 8],
    cr: [Comb; 8],
    al: [Allpass; 4],
    ar: [Allpass; 4],
    nc: usize,
    na: usize,
    damp: f32,
    in_gain: f32,
    out_gain: f32,
}

impl Reverb {
    pub const NEW: Self = Self {
        pre: Line::NEW,
        predelay: None,
        cl: [Comb::NEW; 8],
        cr: [Comb::NEW; 8],
        al: [Allpass::NEW; 4],
        ar: [Allpass::NEW; 4],
        nc: 0,
        na: 0,
        damp: 0.0,
        in_gain: 1.0,
        out_gain: 1.0,
    };

    pub fn init(&mut self, s: &ReverbSettings, pool: &mut Pool) {
        self.nc = s.combs.len().min(8);
        self.na = s.allpasses.len().min(4);
        for k in 0..self.nc {
            self.cl[k].r.take(pool, s.combs[k]);
            self.cr[k].r.take(pool, s.combs[k] + s.spread);
        }
        for k in 0..self.na {
            self.al[k].r.take(pool, s.allpasses[k]);
            self.ar[k].r.take(pool, s.allpasses[k] + s.spread);
        }
        self.predelay = s.predelay;
        if s.predelay.is_some() {
            self.pre.take(pool, 2048);
        }
        self.damp = s.damp;
        self.in_gain = s.in_gain;
        self.out_gain = s.out_gain;
    }

    /// Wet only, stereo; `fb` is the comb feedback (room size).
    #[inline]
    pub fn process(&mut self, x: f32, fb: f32) -> (f32, f32) {
        let x = match self.predelay {
            Some(d) => {
                self.pre.push(x);
                self.pre.read(d) * self.in_gain
            }
            None => x * self.in_gain,
        };
        let mut l = 0.0;
        let mut r = 0.0;
        for c in self.cl[..self.nc].iter_mut() {
            l += c.process(x, fb, self.damp);
        }
        for c in self.cr[..self.nc].iter_mut() {
            r += c.process(x, fb, self.damp);
        }
        for a in self.al[..self.na].iter_mut() {
            l = a.process(l);
        }
        for a in self.ar[..self.na].iter_mut() {
            r = a.process(r);
        }
        (l * self.out_gain, r * self.out_gain)
    }
}
