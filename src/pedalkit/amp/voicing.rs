//! A valve head's circuit values. Each patch keeps its own voicings.

use super::helpers::Peak;

pub struct Voicing {
    // preamp
    /// Input high-pass (Hz).
    pub hp_in_hz: f32,
    /// Gain offset into the preamp (dB).
    pub trim_db: f32,
    /// Triode bias: asymmetric clipping, so even harmonics.
    pub bias: f32,
    /// DC-blocking high-pass after the triode (Hz).
    pub dc_hz: f32,
    /// Interstage low-pass (Hz).
    pub inter_lp_hz: f32,
    /// Second gain stage (dB).
    pub stage2_db: f32,
    // tone stack
    pub bass: Peak,
    pub mid: Peak,
    pub upper: Peak,
    /// Treble control: content above `treble_hz` scaled by `treble_min + treble_span * tone`.
    pub treble_hz: f32,
    pub treble_min: f32,
    pub treble_span: f32,
    // power amp
    /// Drive into the output valves (dB) and their bias (asymmetry).
    pub power_db: f32,
    pub power_bias: f32,
    /// Power supply sag: how far the gain drops when hit hard, and the sag
    /// envelope's attack and release (s).
    pub sag: f32,
    pub sag_attack: f32,
    pub sag_release: f32,
    /// Resonance: a low boost after the clipping.
    pub reso: Peak,
    /// Output trim (linear).
    pub out: f32,
}
