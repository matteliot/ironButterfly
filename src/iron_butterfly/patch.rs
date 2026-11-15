//! The whole patch: a Fuzzrite (or the chimey clean amp, crossfaded) into a
//! Vox-style top boost and Alnico cab, an organ double tracking the guitar,
//! guitar and organ panned apart, into a spring-ish reverb.

use super::fuzzrite::Fuzzrite;
use super::organ::Organ;
use super::presets::*;
use crate::pedalkit::abi::Color;
use crate::pedalkit::dsp::{Biquad, db, tanh};
use crate::pedalkit::eq::EqChain;
use crate::pedalkit::gate::Gate;
use crate::pedalkit::helpers::{Smooth, limit};
use crate::pedalkit::pedal::{Param, Pedal};
use crate::pedalkit::pool::Pool;
use crate::pedalkit::reverb::Reverb;

/// Knob indices.
const FUZZ: usize = 0;
const REVERB_KNOB: usize = 1;
const LEVEL: usize = 2;

pub struct Patch {
    knobs: [f32; 3],
    fuzz_on: bool,
    organ_on: bool,
    g_fuzz: Smooth,
    fuzz_mk: Smooth,
    depth: Smooth,
    out: Smooth,
    wet: Smooth,
    fuzz_mix: Smooth,
    organ_mix: Smooth,
    hp_in: Biquad,
    gate: Gate,
    fuzz: Fuzzrite,
    amp: EqChain<3>,
    cab: EqChain<5>,
    organ: Organ,
    rev: Reverb,
}

impl Patch {
    fn retarget(&mut self) {
        let f = self.knobs[FUZZ];
        self.g_fuzz.set(db(FUZZ_MIN_DB + FUZZ_RANGE_DB * f));
        self.fuzz_mk.set(db(FUZZ_MAKEUP_DB * (FUZZ_DEFAULT - f).max(0.0)));
        self.depth.set(DEPTH_MIN + DEPTH_RANGE * f);
        self.wet.set(REV_MAX_WET * self.knobs[REVERB_KNOB]);
        // Square law: 0 at minimum, unity at 3 o'clock, +5 dB at maximum.
        let v = self.knobs[LEVEL] / 0.75;
        self.out.set(v * v * OUT_TRIM);
    }
}

impl Pedal for Patch {
    const PARAMS: [Param; 3] = [
        // Fuzz gain and Depth together.
        Param { name: "Fuzz", default: 0.7 },
        // Dry to drenched.
        Param { name: "Reverb", default: 0.35 },
        // Output; unity at 3 o'clock.
        Param { name: "Level", default: 0.75 },
    ];
    const POOL: usize = 2 * (1215 + 1293 + 1390 + 1476 + 605 + 480) + 6 * 25;

    const NEW: Self = Self {
        knobs: [Self::PARAMS[0].default, Self::PARAMS[1].default, Self::PARAMS[2].default],
        fuzz_on: true,
        organ_on: true,
        g_fuzz: Smooth::new(1.0),
        fuzz_mk: Smooth::new(1.0),
        depth: Smooth::new(0.5),
        out: Smooth::new(0.0),
        wet: Smooth::new(0.0),
        fuzz_mix: Smooth::new(1.0),
        organ_mix: Smooth::new(1.0),
        hp_in: Biquad::NEW,
        gate: Gate::NEW,
        fuzz: Fuzzrite::NEW,
        amp: EqChain::NEW,
        cab: EqChain::NEW,
        organ: Organ::NEW,
        rev: Reverb::NEW,
    };

    fn init(&mut self, pool: &mut Pool) {
        self.hp_in.highpass(70.0, 0.707);
        self.gate.init(&GATE);
        self.fuzz.init();
        self.amp.init(&AMP);
        self.cab.init(&CAB);
        self.organ.init();
        self.rev.init(&REVERB, pool);
        for s in [&mut self.g_fuzz, &mut self.fuzz_mk, &mut self.depth, &mut self.out, &mut self.wet] {
            s.time(0.02);
        }
        self.fuzz_mix.time(0.012);
        self.organ_mix.time(0.012);
        self.retarget();
        self.fuzz_mix.on(self.fuzz_on);
        self.organ_mix.on(self.organ_on);
        for s in [&mut self.g_fuzz, &mut self.fuzz_mk, &mut self.depth, &mut self.out, &mut self.wet] {
            s.snap();
        }
        self.fuzz_mix.snap();
        self.organ_mix.snap();
    }

    fn set_knob(&mut self, idx: usize, v: f32) {
        self.knobs[idx] = v;
        self.retarget();
    }

    /// Footswitch press: fuzz on/off (off = chimey clean).
    fn press(&mut self) {
        self.fuzz_on = !self.fuzz_on;
        self.fuzz_mix.on(self.fuzz_on);
    }

    /// Footswitch hold: organ double on/off.
    fn hold(&mut self) {
        self.organ_on = !self.organ_on;
        self.organ_mix.on(self.organ_on);
    }

    #[inline]
    fn tick(&mut self, x: f32, _: f32) -> (f32, f32) {
        let g = self.g_fuzz.tick();
        let mk = self.fuzz_mk.tick();
        let depth = self.depth.tick();
        let out = self.out.tick();
        let wet = self.wet.tick();
        let fm = self.fuzz_mix.tick();
        let om = self.organ_mix.tick();
        let x = self.hp_in.process(x);
        let gate = self.gate.process(x);
        // Guitar: fuzz and clean both run, crossfaded.
        let f = self.fuzz.process(x, g, depth, mk) * gate;
        let c = tanh(x * CLEAN_DRIVE) * db(CLEAN_MAKEUP_DB) * 0.1;
        let g = self.cab.process(self.amp.process(c + (f - c) * fm));
        let o = self.organ.process(x, self.gate.env.env) * gate * om;
        let dl = g * GUITAR_PAN.0 + o * ORGAN_PAN.0;
        let dr = g * GUITAR_PAN.1 + o * ORGAN_PAN.1;
        let (wl, wr) = self.rev.process(0.5 * (dl + dr), REV_FEEDBACK);
        let l = dl + (wl - dl) * wet;
        let r = dr + (wr - dr) * wet;
        (limit(l * out), limit(r * out))
    }

    fn led(&self) -> Color {
        match (self.fuzz_on, self.organ_on) {
            (true, true) => Color::Magenta,
            (true, false) => Color::Red,
            (false, true) => Color::LightBlue,
            (false, false) => Color::LightGreen,
        }
    }
}
