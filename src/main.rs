//! iron_butterfly: a Polyend Endless patch for the most "In-A-Gadda-Da-Vida"
//! guitar sound ever: 1968 heavy psych.
//!
//! Signal chain (mono in from the left input, stereo out):
//!   guitar: HPF -> Fuzzrite-style fuzz (two transistor stages blended
//!           out of phase: thin, raspy, nasal) or a chimey clean -> Vox-style
//!           top-boost amp EQ -> cab
//!   organ double: the guitar is pitch-tracked (Schmitt trigger on a
//!           low-passed copy) into square waves at the played pitch and an
//!           octave below (16' + 8' combo-organ stops) with a vibrato wobble. It follows
//!           the guitar's envelope, so the riff is doubled by an "organ"
//!   both -> 1968-style reverb (Schroeder/Freeverb-style, stereo) -> level
//!   Guitar sits left-of-centre, organ right-of-centre.
//!
//! Controls
//!   Left knob   Fuzz    (gain and Depth of the fuzz together)
//!   Mid knob    Reverb  (dry -> drenched)
//!   Right knob  Level   (output; unity at 3 o'clock)
//!   Footswitch press  fuzz on/off (off = chimey clean)
//!   Footswitch hold   organ double on/off
//!   LED  fuzz+organ: magenta | fuzz: red | clean+organ: light blue | clean: light green
//!
//! The sound lives in `iron_butterfly` (built from `pedalkit` blocks set by its
//! `presets`); the entry points come from `pedalkit::endless_patch!`.
//! Host builds (`cargo sim`) render a demo riff through the same code; see `host`.

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

/// Reusable blocks (effects, DSP, the Endless ABI); see its docs.
mod pedalkit;

/// The pedal itself.
mod iron_butterfly;

endless_patch!(kit = crate::pedalkit, pedal = crate::iron_butterfly::Patch);

#[cfg(not(target_os = "none"))]
mod host;

#[cfg(not(target_os = "none"))]
fn main() {
    host::main();
}
