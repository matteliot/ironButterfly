//! pedalkit: reusable building blocks for Polyend Endless patches.
//!
//! Nothing in here knows about a particular pedal: every effect takes a
//! settings struct, and each patch keeps its own presets. Modules only refer
//! to each other through relative paths (`super::`), so this directory can be
//! lifted out into its own crate unchanged.
//!
//!   abi       the Endless patch ABI (FxPatchSDK 0x000B)
//!   pedal     the `Pedal` trait and the `Runner` the entry points drive
//!   endless   `endless_patch!`: entry points, header and panic handler for a `Pedal`
//!   pool      delay memory (`Pool`, `Line`, `Ring`), static or the working buffer
//!   global    a static cell for state shared with the firmware entry points
//!   dsp       f32-only math, biquads, one-pole, delay lines
//!   helpers   Smooth, Peak, Clip2x, Phase; limit, soft_limit, hard, biased
//!   env       Follower (attack/release), Onset (pick detector), ExpEnvelope
//!   gate      noise gate with hysteresis and a smoothed gain
//!   eq        Eq stages and EqChain: cabinets, voicings, any biquad chain
//!   clean     clean amp: high-pass, peak compressor, soft tube edge
//!   amp       a valve head: Preamp -> ToneStack -> PowerAmp, set by a Voicing
//!   muff      Big Muff-style fuzz, crossfaded in
//!   wah       a wah: parked or swept, faded in
//!   reverb    Freeverb-style stereo reverb (pool-backed)
//!   feedback  amp feedback loop: delayed, band-passed, blooming
//!   voice     spoken prompts (mu-law) with ducking
//!   tools/    build-endl.sh, check_endl.py, make-prompts.sh
//!   guitar    (host only) a modelled electric guitar for demo renders
//!   host      (host only) the demo renderer: score -> entry points -> stats + WAV

// A kit: each patch uses only some of it.
#![allow(dead_code)]

pub mod abi;
pub mod amp;
pub mod clean;
pub mod dsp;
pub mod endless;
pub mod env;
pub mod eq;
pub mod feedback;
pub mod gate;
pub mod global;
pub mod helpers;
pub mod muff;
pub mod pedal;
pub mod pool;
pub mod reverb;
pub mod voice;
pub mod wah;

#[cfg(not(target_os = "none"))]
pub mod guitar;
#[cfg(not(target_os = "none"))]
pub mod host;
