//! Iron Butterfly's settings: every number that shapes the sound.

use crate::pedalkit::eq::Eq;
use crate::pedalkit::gate::GateSettings;
use crate::pedalkit::reverb::ReverbSettings;

/// Fuzz gain (dB) at knob 0 and the extra at knob 1; Depth (blend towards
/// the second transistor) follows the same knob. The range starts low enough
/// that backing the knob off cleans up chords and ringing notes (less
/// intermodulation hash), like rolling back a real fuzz.
pub const FUZZ_MIN_DB: f32 = 6.0;
pub const FUZZ_RANGE_DB: f32 = 34.0;
pub const DEPTH_MIN: f32 = 0.45;
pub const DEPTH_RANGE: f32 = 0.5;
/// Second transistor stage gain, which follows Depth: MIN + RANGE * depth.
pub const STAGE2_MIN: f32 = 1.5;
pub const STAGE2_RANGE: f32 = 4.5;
/// Below the default Fuzz setting, this much makeup (dB per unit of knob
/// below it) keeps the volume curve where it was before the wider range.
pub const FUZZ_DEFAULT: f32 = 0.7;
pub const FUZZ_MAKEUP_DB: f32 = 7.0;

/// Clean: drive into the amp's chimey breakup, and makeup gain (dB) so it
/// sits a little under the fuzz.
pub const CLEAN_DRIVE: f32 = 2.5;
pub const CLEAN_MAKEUP_DB: f32 = 36.0;

/// Organ double: tracking low-pass (Hz), Schmitt threshold (fraction of the
/// tracked envelope), stop mix, level, vibrato rate (Hz) and depth.
pub const TRACK_HZ: f32 = 320.0;
pub const TRACK_THRESH: f32 = 0.25;
pub const ORGAN_SUB: f32 = 0.55;
pub const ORGAN_FUND: f32 = 0.45;
pub const ORGAN_GAIN: f32 = 19.0;
pub const ORGAN_VIB_HZ: f32 = 6.2;
pub const ORGAN_VIB_DEPTH: f32 = 0.08;

/// Stereo placement (left gain, right gain).
pub const GUITAR_PAN: (f32, f32) = (1.0, 0.6);
pub const ORGAN_PAN: (f32, f32) = (0.6, 1.0);

/// Reverb: Freeverb's first four combs and two allpasses (48 kHz), comb
/// feedback and damping, and the max wet at Reverb knob 1.
pub const REVERB: ReverbSettings = ReverbSettings {
    combs: &[1215, 1293, 1390, 1476],
    allpasses: &[605, 480],
    spread: 25,
    damp: 0.35,
    in_gain: 0.25,
    out_gain: 1.0,
    predelay: None,
};
pub const REV_FEEDBACK: f32 = 0.83;
pub const REV_MAX_WET: f32 = 0.6;

/// Noise gate on the input envelope.
pub const GATE: GateSettings =
    GateSettings { open: 0.0012, close: 0.0006, attack: 0.001, release: 0.08, fade_in: 0.002, fade_out: 0.06 };

/// Output trim: unity Level puts the fuzz + organ riff around -14 dB RMS.
pub const OUT_TRIM: f32 = 0.3;

/// Vox-style top boost: cut low mids, chime up top.
pub const AMP: [Eq; 3] = [Eq::Hp(90.0, 0.707), Eq::Peak(250.0, 0.9, -2.0), Eq::Peak(2800.0, 1.0, 4.0)];
/// Alnico 12s: little deep bass, bright upper mids, soft top.
pub const CAB: [Eq; 5] =
    [Eq::Hp(95.0, 0.707), Eq::Peak(1200.0, 1.0, 2.0), Eq::Peak(3500.0, 1.5, 3.0), Eq::Lp(6500.0, 0.75), Eq::Lp(9000.0, 0.6)];
