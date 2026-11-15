//! The `cargo sim` demo: its score (the rendering itself is `pedalkit::host`).
//!
//! Plays an original D-minor riff in a 1968 heavy-psych style at 112 bpm on a
//! modelled electric guitar (see `guitar`: exact tuning, pick and pickup, vibrato,
//! human timing), drives the patch's entry points the
//! way the firmware does (128-sample blocks, footswitch events at set times)
//! and writes a stereo float WAV. Prints per-section level stats and fails on
//! NaN or clipping.

use crate::pedalkit::guitar::Note;
use crate::pedalkit::host::{Ev, Score};

const D2: f32 = 73.42;
const F2: f32 = 87.31;
const G2: f32 = 98.0;
const A2: f32 = 110.0;
const C3: f32 = 130.81;
const D3: f32 = 146.83;
const F3: f32 = 174.61;
const A3: f32 = 220.0;
const D4: f32 = 293.66;

enum Local {
    /// Play these notes, strummed this many seconds apart, releasing what's still ringing.
    Play(Vec<Note>, f32),
    Press,
    Hold,
}

fn local_score() -> (Vec<(f64, Local, &'static str)>, f64) {
    let mut ev = vec![];
    let e = 60.0 / 112.0 / 2.0; // eighth note
    let mut t = 0.1;
    // Small, repeatable timing slop: nobody plays exactly on the grid.
    let mut seed: u32 = 0x1357_9bdf;
    let mut slop = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f64 / u32::MAX as f64 * 2.0 - 1.0) * 0.007
    };
    // Original single-note riff: (note, length in eighths). Long notes get vibrato.
    let riff = [(D2, 1.0), (D2, 1.0), (A2, 1.0), (C3, 1.0), (D3, 2.0), (C3, 1.0), (A2, 1.0), (G2, 2.0), (F2, 1.0), (G2, 1.0), (A2, 4.0)];
    let play = |ev: &mut Vec<(f64, Local, &'static str)>, t: &mut f64, times: usize, slop: &mut dyn FnMut() -> f64| {
        for _ in 0..times {
            for (i, (f, len)) in riff.into_iter().enumerate() {
                let n = Note::new(f).vel(if i % 2 == 0 { 0.9 } else { 0.75 });
                let n = if len >= 2.0 { n.vib() } else { n };
                ev.push((*t + slop(), Local::Play(vec![n], 0.0), ""));
                *t += len * e;
            }
        }
    };
    play(&mut ev, &mut t, 2, &mut slop);
    ev.push((t, Local::Hold, "fuzz only (organ off)"));
    play(&mut ev, &mut t, 1, &mut slop);
    ev.push((t, Local::Press, "clean + reverb"));
    ev.push((t + 0.001, Local::Hold, ""));
    for _ in 0..2 {
        for n in [[D3, F3], [A3, D4], [C3, F3], [A3, D4]] {
            ev.push((t + 0.002 + slop().abs(), Local::Play(n.iter().map(|f| Note::new(*f).vel(0.7).vib()).collect(), 0.012), ""));
            t += 2.0 * e;
        }
    }
    ev.push((t, Local::Press, "fuzz + organ again"));
    play(&mut ev, &mut t, 1, &mut slop);
    (ev, t + 1.5)
}

/// The score as pedalkit events.
pub fn score() -> (Score, f64) {
    let (ev, total) = local_score();
    (ev.into_iter().map(|(t, e, l)| (t as f64, e.into(), l)).collect(), total as f64)
}

impl From<Local> for Ev {
    fn from(e: Local) -> Ev {
        match e {
            Local::Play(notes, strum) => Ev::Play(notes, strum, false),
            Local::Press => Ev::Press,
            Local::Hold => Ev::Hold,
        }
    }
}
