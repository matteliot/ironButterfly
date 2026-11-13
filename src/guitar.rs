//! Host-only electric guitar model for the demo render: an extended Karplus-Strong (Jaffe-Smith) string per note, played like a person would.
//!
//! What makes it sound like a guitar rather than a plucked-noise "MIDI" tone:
//! - Exact tuning: the loop delay is fractional (4-point Lagrange interpolation) with the loss filter's group delay subtracted, so every note is in tune, and the pitch can move smoothly (vibrato,
//!   bends, slides).
//! - Two polarizations per string, slightly detuned with different decays, so notes beat and decay unevenly like a real string.
//! - Pick: the excitation is band-limited noise shaped by the picking position (a comb), its brightness following how hard the note is picked, plus a short pick click. Hard picks go a few cents
//!   sharp at the attack and settle (tension modulation).
//! - Pickup: a bridge humbucker heard as a comb at its position along the string, then its resonant low-pass (~4.3 kHz) into the cable.
//! - Decay: low strings ring longer than high ones; palm mutes are short and dark; releasing a note damps it in ~50 ms instead of cutting it.
//! - Playing: vibrato that fades in on held notes, bends and slides, strummed chords, and small random timing and velocity differences (see `Player`).

use crate::dsp::Biquad;

pub const SR: f32 = 48_000.0;
const RING: usize = 4096; // power of two, longer than the lowest note's period (E2 = 582 samples)

/// One note: its pitch, how hard it's picked and how it's played.
#[derive(Clone, Copy)]
pub struct Note {
    pub f: f32,
    pub vel: f32,
    /// Time for the string to ring down 60 dB while held (s).
    pub t60: f32,
    pub mute: bool,
    pub vib: bool,
    /// (start s, duration s, cents)
    pub bend: Option<(f32, f32, f32)>,
    /// Slide in from this many semitones below, over this many seconds.
    pub slide: Option<(f32, f32)>,
}

impl Note {
    pub fn new(f: f32) -> Note {
        // Low strings ring for seconds, high ones much less.
        let t60 = (6.0 * (82.41 / f).sqrt()).clamp(1.8, 6.0);
        Note { f, vel: 0.8, t60, mute: false, vib: false, bend: None, slide: None }
    }
    /// Palm muted: short, dark and a little softer.
    pub fn pm(mut self) -> Note {
        self.mute = true;
        self.t60 = 0.18;
        self.vel *= 0.85;
        self
    }
    /// Overrides how long it rings while held (s): e.g. a fret-hand-muted "chk" is a few tens of milliseconds.
    pub fn t60(mut self, t60: f32) -> Note {
        self.t60 = t60;
        self
    }
    pub fn vib(mut self) -> Note {
        self.vib = true;
        self
    }
    pub fn vel(mut self, v: f32) -> Note {
        self.vel = v;
        self
    }
    pub fn bend(mut self, start: f32, dur: f32, cents: f32) -> Note {
        self.bend = Some((start, dur, cents));
        self
    }
    pub fn slide(mut self, semitones: f32, dur: f32) -> Note {
        self.slide = Some((semitones, dur));
        self
    }
}

/// One polarization of a string: a fractional delay line with a one-zero loss filter in the loop.
struct Str {
    buf: Vec<f32>,
    w: usize,
    prev: f32,
    /// Loss filter tap (0 = no low-pass, 0.5 = darkest); its group delay is `s` samples.
    s: f32,
}

impl Str {
    fn new(s: f32) -> Str {
        Str { buf: vec![0.0; RING], w: 0, prev: 0.0, s }
    }

    /// The sample written `k` steps ago (k >= 1).
    fn at(&self, k: usize) -> f32 {
        self.buf[(self.w + 1).wrapping_sub(k) & (RING - 1)]
    }

    /// The sample `d` steps ago (d >= 2), 4-point Lagrange interpolated.
    fn read(&self, d: f32) -> f32 {
        let i = d as usize;
        let x = d - i as f32;
        let (a, b, c, e) = (self.at(i - 1), self.at(i), self.at(i + 1), self.at(i + 2));
        let h0 = -x * (x - 1.0) * (x - 2.0) / 6.0;
        let h1 = (x + 1.0) * (x - 1.0) * (x - 2.0) / 2.0;
        let h2 = -(x + 1.0) * x * (x - 2.0) / 2.0;
        let h3 = (x + 1.0) * x * (x - 1.0) / 6.0;
        a * h0 + b * h1 + c * h2 + e * h3
    }

    /// One sample around the loop for a period of `period` samples, with loop gain `g`; returns what was written.
    fn step(&mut self, period: f32, g: f32, input: f32) -> f32 {
        let y = self.read((period - self.s).max(2.0));
        let l = g * ((1.0 - self.s) * y + self.s * self.prev);
        self.prev = y;
        self.w = (self.w + 1) & (RING - 1);
        self.buf[self.w] = l + input;
        l + input
    }
}

/// A sounding note.
pub struct Voice {
    note: Note,
    a: Str,
    b: Str,
    exc: Vec<f32>,
    click: Vec<f32>,
    pickup: f32,
    t: usize,
    released: Option<usize>,
}

/// Bridge humbucker's distance from the bridge, as a fraction of the string.
const PICKUP_POS: f32 = 0.075;
/// Second polarization: detune (cents), level, and its decay relative to the first.
const POL_DETUNE: f32 = 0.7;
const POL_LEVEL: f32 = 0.45;
const POL_DECAY: f32 = 0.8;
/// Vibrato: rate (Hz), depth (cents), and its fade-in (delay, ramp) on a held note (s).
const VIB_HZ: f32 = 5.3;
const VIB_CENTS: f32 = 18.0;
const VIB_DELAY: f32 = 0.22;
const VIB_RAMP: f32 = 0.35;
/// Ring-down time once a note is released (fretting hand lifts off).
const RELEASE_T60: f32 = 0.06;
/// Overall level into the patch, like a humbucker's output.
const LEVEL: f32 = 0.55;

impl Voice {
    /// `rnd` gives uniform values in -1..1; `pick_pos` is where along the string it's picked (fraction from the bridge).
    pub fn new(note: Note, pick_pos: f32, rnd: &mut impl FnMut() -> f32) -> Voice {
        let period = SR / note.f;
        let n = period.round() as usize;
        // Band-limited noise: brighter the harder it's picked, dark when palm muted.
        let fc = (1200.0 + 6000.0 * note.vel) * if note.mute { 0.45 } else { 1.0 };
        let k = 1.0 - (-core::f32::consts::TAU * fc / SR).exp();
        let mut z = 0.0;
        let raw: Vec<f32> = (0..n)
            .map(|_| {
                z += k * (rnd() - z);
                z
            })
            .collect();
        // Picking position: a comb that notches the harmonics with a node there.
        let p = ((pick_pos * period).round() as usize).max(1);
        let mut exc: Vec<f32> = (0..n).map(|i| raw[i] - if i >= p { raw[i - p] } else { 0.0 }).collect();
        let mean = exc.iter().sum::<f32>() / n as f32;
        let rms = (exc.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n as f32).sqrt().max(1e-9);
        for v in exc.iter_mut() {
            *v = (*v - mean) / rms * note.vel * 0.12;
        }
        // Pick click: a couple of milliseconds of bright noise.
        let mut last = 0.0;
        let click = (0..96)
            .map(|i| {
                let v = rnd();
                let hp = v - last;
                last = v;
                hp * note.vel * 0.015 * (-(i as f32) / 20.0).exp()
            })
            .collect();
        let s = if note.mute { 0.5 } else { 0.18 + 0.12 * (1.0 - note.vel) };
        Voice { note, a: Str::new(s), b: Str::new(s), exc, click, pickup: PICKUP_POS, t: 0, released: None }
    }

    pub fn release(&mut self) {
        if self.released.is_none() {
            self.released = Some(self.t);
        }
    }

    /// Silent and released: safe to drop.
    pub fn done(&self) -> bool {
        self.released.is_some_and(|r| self.t > r + (SR * 0.4) as usize) || self.t as f32 > SR * self.note.t60 * 1.5
    }

    fn cents(&self, ts: f32) -> f32 {
        let n = &self.note;
        // Hard picks start a little sharp and settle.
        let mut c = 7.0 * n.vel * (-ts / 0.05).exp();
        if n.vib {
            let depth = ((ts - VIB_DELAY) / VIB_RAMP).clamp(0.0, 1.0);
            c += VIB_CENTS * depth * (core::f32::consts::TAU * VIB_HZ * ts).sin();
        }
        if let Some((start, dur, cents)) = n.bend {
            c += cents * smoothstep((ts - start) / dur);
        }
        if let Some((semis, dur)) = n.slide {
            c -= 100.0 * semis * (1.0 - smoothstep(ts / dur));
        }
        c
    }

    pub fn tick(&mut self) -> f32 {
        let ts = self.t as f32 / SR;
        let f = self.note.f * (self.cents(ts) / 1200.0).exp2();
        let period = SR / f;
        let t60 = if self.released.is_some() { RELEASE_T60.min(self.note.t60) } else { self.note.t60 };
        // Loop gain per period for a 60 dB ring-down in t60.
        let g = 10f32.powf(-3.0 / (t60 * f));
        let gb = 10f32.powf(-3.0 / (POL_DECAY * t60 * f));
        let input = self.exc.get(self.t).copied().unwrap_or(0.0);
        let va = self.a.step(period, g, input);
        let vb = self.b.step(period / (POL_DETUNE / 1200.0).exp2(), gb, input);
        // The pickup hears the string at its position: a comb at that fraction of the period.
        let p = (self.pickup * period).max(2.0);
        let out = (va - self.a.read(p)) + POL_LEVEL * (vb - self.b.read(p));
        let click = self.click.get(self.t).copied().unwrap_or(0.0);
        self.t += 1;
        (out + click) * LEVEL
    }
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Plays notes like a person: releases what's ringing when new notes come in (unless told to let them ring), strums chords low to high, and puts the result through the pickup's resonance.
pub struct Player {
    voices: Vec<Voice>,
    pending: Vec<(usize, Note, f32)>,
    lp: Biquad,
    hp: Biquad,
    t: usize,
    seed: u32,
}

impl Player {
    pub fn new() -> Player {
        let mut lp = Biquad::NEW;
        lp.lowpass(4300.0, 1.3); // humbucker + cable resonance
        let mut hp = Biquad::NEW;
        hp.highpass(60.0, 0.707);
        Player { voices: vec![], pending: vec![], lp, hp, t: 0, seed: 0x2468_ace1 }
    }

    /// Uniform in -1..1.
    pub fn rnd(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    /// Plays `notes` now, strummed `strum` seconds apart (low to high). `ring` lets notes already sounding carry on.
    pub fn play(&mut self, notes: &[Note], strum: f32, ring: bool) {
        if !ring {
            self.voices.iter_mut().for_each(Voice::release);
        }
        for (i, n) in notes.iter().enumerate() {
            // A human never hits two notes at exactly the same strength.
            let mut n = *n;
            n.vel = (n.vel * (1.0 + 0.12 * self.rnd())).clamp(0.2, 1.0);
            let jitter = 0.0015 * self.rnd().abs();
            let at = self.t + ((i as f32 * strum + jitter) * SR) as usize;
            let pick = 0.16 + 0.04 * self.rnd();
            self.pending.push((at, n, pick));
        }
    }

    /// Releases everything (the fretting hand lets go).
    pub fn release_all(&mut self) {
        self.voices.iter_mut().for_each(Voice::release);
    }

    pub fn tick(&mut self) -> f32 {
        let t = self.t;
        let mut i = 0;
        while i < self.pending.len() {
            if self.pending[i].0 <= t {
                let (_, n, pick) = self.pending.remove(i);
                let mut seed = self.seed;
                let mut rnd = move || {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    seed as f32 / u32::MAX as f32 * 2.0 - 1.0
                };
                let v = Voice::new(n, pick, &mut rnd);
                self.rnd();
                self.voices.push(v);
            } else {
                i += 1;
            }
        }
        self.voices.retain(|v| !v.done());
        let x: f32 = self.voices.iter_mut().map(Voice::tick).sum();
        self.t += 1;
        self.hp.process(self.lp.process(x))
    }
}
