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
//! Host builds (`cargo sim`) render a demo riff through the same code; see `host`.

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg_attr(not(target_os = "none"), allow(dead_code))]
mod abi;
#[allow(dead_code)]
mod dsp;

use core::cell::UnsafeCell;
use core::f32::consts::TAU;

use abi::*;
use dsp::*;

// ---------------------------------------------------------------- voicing

/// Fuzz gain (dB) at knob 0 and the extra at knob 1; Depth (blend towards
/// the second transistor) follows the same knob.
const FUZZ_MIN_DB: f32 = 18.0;
const FUZZ_RANGE_DB: f32 = 22.0;
const DEPTH_MIN: f32 = 0.45;
const DEPTH_RANGE: f32 = 0.5;
/// Second transistor stage gain.
const STAGE2_GAIN: f32 = 6.0;

/// Clean: drive into the amp's chimey breakup, and makeup gain (dB) so it
/// sits a little under the fuzz.
const CLEAN_DRIVE: f32 = 2.5;
const CLEAN_MAKEUP_DB: f32 = 36.0;

/// Organ double: tracking low-pass (Hz), Schmitt threshold (fraction of the
/// tracked envelope), stop mix, level, vibrato rate (Hz) and depth.
const TRACK_HZ: f32 = 320.0;
const TRACK_THRESH: f32 = 0.25;
const ORGAN_SUB: f32 = 0.55;
const ORGAN_FUND: f32 = 0.45;
const ORGAN_GAIN: f32 = 19.0;
const ORGAN_VIB_HZ: f32 = 6.2;
const ORGAN_VIB_DEPTH: f32 = 0.08;

/// Stereo placement (left gain, right gain).
const GUITAR_PAN: (f32, f32) = (1.0, 0.6);
const ORGAN_PAN: (f32, f32) = (0.6, 1.0);

/// Reverb: comb feedback, comb damping, max wet at Reverb knob 1.
const REV_FEEDBACK: f32 = 0.83;
const REV_DAMP: f32 = 0.35;
const REV_MAX_WET: f32 = 0.6;

/// Noise gate thresholds on the input envelope (linear), with hysteresis.
const GATE_OPEN: f32 = 0.0012;
const GATE_CLOSE: f32 = 0.0006;

/// Output trim: unity Level puts the fuzz + organ riff around -14 dB RMS.
const OUT_TRIM: f32 = 0.3;

// ---------------------------------------------------------------- reverb parts

/// Lowpass-feedback comb filter (Freeverb style), length N samples.
struct Comb<const N: usize> {
    buf: [f32; N],
    i: usize,
    z: f32,
}

impl<const N: usize> Comb<N> {
    const NEW: Self = Self { buf: [0.0; N], i: 0, z: 0.0 };
    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.buf[self.i];
        self.z = y + (self.z - y) * REV_DAMP;
        self.buf[self.i] = x + self.z * REV_FEEDBACK;
        self.i = if self.i + 1 == N { 0 } else { self.i + 1 };
        y
    }
}

/// Schroeder allpass, length N samples.
struct Allpass<const N: usize> {
    buf: [f32; N],
    i: usize,
}

impl<const N: usize> Allpass<N> {
    const NEW: Self = Self { buf: [0.0; N], i: 0 };
    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let b = self.buf[self.i];
        self.buf[self.i] = x + b * 0.5;
        self.i = if self.i + 1 == N { 0 } else { self.i + 1 };
        b - x
    }
}

/// Freeverb's tunings scaled to 48 kHz; the right channel is spread by 25.
struct Reverb {
    l: (Comb<1215>, Comb<1293>, Comb<1390>, Comb<1476>, Allpass<605>, Allpass<480>),
    r: (Comb<1240>, Comb<1318>, Comb<1415>, Comb<1501>, Allpass<630>, Allpass<505>),
}

impl Reverb {
    const NEW: Self = Self {
        l: (Comb::NEW, Comb::NEW, Comb::NEW, Comb::NEW, Allpass::NEW, Allpass::NEW),
        r: (Comb::NEW, Comb::NEW, Comb::NEW, Comb::NEW, Allpass::NEW, Allpass::NEW),
    };

    #[inline]
    fn process(&mut self, x: f32) -> (f32, f32) {
        let x = x * 0.25;
        let l = &mut self.l;
        let yl = l.5.process(l.4.process(l.0.process(x) + l.1.process(x) + l.2.process(x) + l.3.process(x)));
        let r = &mut self.r;
        let yr = r.5.process(r.4.process(r.0.process(x) + r.1.process(x) + r.2.process(x) + r.3.process(x)));
        (yl, yr)
    }
}

// ---------------------------------------------------------------- state

struct Ib {
    ready: bool,
    error: bool,
    // knobs, 0..1
    fuzz: f32,
    reverb: f32,
    level: f32,
    fuzz_on: bool,
    organ_on: bool,
    // smoothed values and targets
    g_fuzz: f32,
    g_fuzz_t: f32,
    depth: f32,
    depth_t: f32,
    out_g: f32,
    out_t: f32,
    wet: f32,
    wet_t: f32,
    fuzz_mix: f32,
    organ_mix: f32,
    // guitar filters
    hp_in: Biquad,
    fuzz_hp: Biquad,
    fuzz_lp: Biquad,
    fuzz_nasal: Biquad,
    amp: [Biquad; 3],
    cab: [Biquad; 5],
    u1: f32,
    // tracking / organ
    trk: [Biquad; 2],
    trk_env: f32,
    high: bool,
    sub: bool,
    organ_env: f32,
    organ_lp: Biquad,
    organ_hp: Biquad,
    vib_ph: f32,
    // dynamics
    env: f32,
    gate: f32,
    gate_open: bool,
    // coefficients
    c_param: f32,
    c_switch: f32,
    c_att: f32,
    c_rel: f32,
    c_gate_open: f32,
    c_gate_close: f32,
    c_org_att: f32,
    c_org_rel: f32,
}

impl Ib {
    const NEW: Self = Self {
        ready: false,
        error: false,
        fuzz: 0.7,
        reverb: 0.35,
        level: 0.75,
        fuzz_on: true,
        organ_on: true,
        g_fuzz: 1.0,
        g_fuzz_t: 1.0,
        depth: 0.5,
        depth_t: 0.5,
        out_g: 0.0,
        out_t: 0.0,
        wet: 0.0,
        wet_t: 0.0,
        fuzz_mix: 1.0,
        organ_mix: 1.0,
        hp_in: Biquad::NEW,
        fuzz_hp: Biquad::NEW,
        fuzz_lp: Biquad::NEW,
        fuzz_nasal: Biquad::NEW,
        amp: [Biquad::NEW; 3],
        cab: [Biquad::NEW; 5],
        u1: 0.0,
        trk: [Biquad::NEW; 2],
        trk_env: 0.0,
        high: false,
        sub: false,
        organ_env: 0.0,
        organ_lp: Biquad::NEW,
        organ_hp: Biquad::NEW,
        vib_ph: 0.0,
        env: 0.0,
        gate: 0.0,
        gate_open: false,
        c_param: 1.0,
        c_switch: 1.0,
        c_att: 1.0,
        c_rel: 1.0,
        c_gate_open: 1.0,
        c_gate_close: 1.0,
        c_org_att: 1.0,
        c_org_rel: 1.0,
    };

    fn init(&mut self) {
        self.hp_in.highpass(70.0, 0.707);
        self.fuzz_hp.highpass(160.0, 0.707); // Fuzzrites are thin
        self.fuzz_lp.lowpass(6500.0, 0.707);
        self.fuzz_nasal.peak(1800.0, 1.2, 4.0);
        // Vox-style top boost: cut low mids, chime up top.
        self.amp[0].highpass(90.0, 0.707);
        self.amp[1].peak(250.0, 0.9, -2.0);
        self.amp[2].peak(2800.0, 1.0, 4.0);
        // Alnico 12s: little deep bass, bright upper mids, soft top.
        self.cab[0].highpass(95.0, 0.707);
        self.cab[1].peak(1200.0, 1.0, 2.0);
        self.cab[2].peak(3500.0, 1.5, 3.0);
        self.cab[3].lowpass(6500.0, 0.75);
        self.cab[4].lowpass(9000.0, 0.6);
        self.trk[0].lowpass(TRACK_HZ, 0.707);
        self.trk[1].lowpass(TRACK_HZ, 0.707);
        self.organ_lp.lowpass(2500.0, 0.707);
        self.organ_hp.highpass(30.0, 0.707);
        self.c_param = coef(0.02);
        self.c_switch = coef(0.012);
        self.c_att = coef(0.001);
        self.c_rel = coef(0.08);
        self.c_gate_open = coef(0.002);
        self.c_gate_close = coef(0.06);
        self.c_org_att = coef(0.004);
        self.c_org_rel = coef(0.12);
        self.retarget();
        self.g_fuzz = self.g_fuzz_t;
        self.depth = self.depth_t;
        self.out_g = self.out_t;
        self.wet = self.wet_t;
        self.fuzz_mix = if self.fuzz_on { 1.0 } else { 0.0 };
        self.organ_mix = if self.organ_on { 1.0 } else { 0.0 };
        self.ready = true;
    }

    fn retarget(&mut self) {
        self.g_fuzz_t = db(FUZZ_MIN_DB + FUZZ_RANGE_DB * self.fuzz);
        self.depth_t = DEPTH_MIN + DEPTH_RANGE * self.fuzz;
        self.wet_t = REV_MAX_WET * self.reverb;
        // Square law: 0 at minimum, unity at 3 o'clock, +5 dB at maximum.
        let v = self.level / 0.75;
        self.out_t = v * v * OUT_TRIM;
    }

    /// Fuzzrite-style: two clipping transistor stages, blended out of phase by Depth.
    #[inline]
    fn fuzzrite(&mut self, x: f32) -> f32 {
        let u = self.fuzz_hp.process(x) * self.g_fuzz;
        let a = 0.5 * (stage1(0.5 * (u + self.u1)) + stage1(u));
        self.u1 = u;
        let b = (a * STAGE2_GAIN).clamp(-1.0, 1.0);
        let y = b * self.depth - 0.5 * a * (1.0 - self.depth);
        self.fuzz_nasal.process(self.fuzz_lp.process(y))
    }

    /// Organ double: squares at the tracked pitch and an octave below.
    #[inline]
    fn organ(&mut self, x: f32, in_env: f32) -> f32 {
        let t = self.trk[0].process(x);
        let t = self.trk[1].process(t);
        let a = t.abs();
        self.trk_env += (a - self.trk_env) * if a > self.trk_env { self.c_att } else { self.c_rel };
        let th = TRACK_THRESH * self.trk_env;
        if !self.high && t > th {
            self.high = true;
            self.sub = !self.sub;
        } else if self.high && t < -th {
            self.high = false;
        }
        let fund = if self.high { 1.0 } else { -1.0 };
        let sub = if self.sub { 1.0 } else { -1.0 };
        self.organ_env += (in_env - self.organ_env) * if in_env > self.organ_env { self.c_org_att } else { self.c_org_rel };
        // Combo-organ wobble, as a gentle amplitude vibrato.
        self.vib_ph += ORGAN_VIB_HZ / SR;
        if self.vib_ph >= 1.0 {
            self.vib_ph -= 1.0;
        }
        let vib = 1.0 + ORGAN_VIB_DEPTH * sin(TAU * self.vib_ph);
        let o = (ORGAN_SUB * sub + ORGAN_FUND * fund) * self.organ_env * ORGAN_GAIN * vib;
        self.organ_hp.process(self.organ_lp.process(o))
    }

    #[inline]
    fn tick(&mut self, rev: &mut Reverb, x: f32) -> (f32, f32) {
        self.g_fuzz += (self.g_fuzz_t - self.g_fuzz) * self.c_param;
        self.depth += (self.depth_t - self.depth) * self.c_param;
        self.out_g += (self.out_t - self.out_g) * self.c_param;
        self.wet += (self.wet_t - self.wet) * self.c_param;
        let ft = if self.fuzz_on { 1.0 } else { 0.0 };
        self.fuzz_mix += (ft - self.fuzz_mix) * self.c_switch;
        let ot = if self.organ_on { 1.0 } else { 0.0 };
        self.organ_mix += (ot - self.organ_mix) * self.c_switch;

        let x = self.hp_in.process(x);
        let a = x.abs();
        self.env += (a - self.env) * if a > self.env { self.c_att } else { self.c_rel };
        if self.env > GATE_OPEN {
            self.gate_open = true;
        } else if self.env < GATE_CLOSE {
            self.gate_open = false;
        }
        let (gt, gc) = if self.gate_open { (1.0, self.c_gate_open) } else { (0.0, self.c_gate_close) };
        self.gate += (gt - self.gate) * gc;

        // Guitar: fuzz and clean both run, crossfaded.
        let f = self.fuzzrite(x) * self.gate;
        let c = tanh(x * CLEAN_DRIVE) * db(CLEAN_MAKEUP_DB) * 0.1;
        let mut g = c + (f - c) * self.fuzz_mix;
        for b in self.amp.iter_mut() {
            g = b.process(g);
        }
        for b in self.cab.iter_mut() {
            g = b.process(g);
        }
        let env = self.env;
        let o = self.organ(x, env) * self.gate * self.organ_mix;

        let dl = g * GUITAR_PAN.0 + o * ORGAN_PAN.0;
        let dr = g * GUITAR_PAN.1 + o * ORGAN_PAN.1;
        let (wl, wr) = rev.process(0.5 * (dl + dr));
        let l = dl + (wl - dl) * self.wet;
        let r = dr + (wr - dr) * self.wet;
        (limit(l * self.out_g), limit(r * self.out_g))
    }

    fn led(&self) -> Color {
        if self.error {
            return Color::Red;
        }
        match (self.fuzz_on, self.organ_on) {
            (true, true) => Color::Magenta,
            (true, false) => Color::Red,
            (false, true) => Color::LightBlue,
            (false, false) => Color::LightGreen,
        }
    }
}

/// First Fuzzrite transistor: starved and asymmetric (soft on top, hard on the bottom).
#[inline]
fn stage1(u: f32) -> f32 {
    if u > 0.0 { tanh(u) } else { (u * 1.4).max(-0.7) }
}

/// Output soft limiter, never reaching ±1.
#[inline]
fn limit(x: f32) -> f32 {
    tanh(x) * 0.98
}

/// All patch state. The firmware calls every entry point from the audio
/// thread (SDK: "no need to synchronize"). The reverb's buffers are all
/// zero, so they live in .bss and don't bloat the patch file.
struct Global<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Global<T> {}
static STATE: Global<Ib> = Global(UnsafeCell::new(Ib::NEW));
static REVERB: Global<Reverb> = Global(UnsafeCell::new(Reverb::NEW));

#[allow(clippy::mut_from_ref)]
fn state() -> &'static mut Ib {
    // SAFETY: single-threaded access from the firmware's audio thread.
    unsafe { &mut *STATE.0.get() }
}

#[allow(clippy::mut_from_ref)]
fn reverb() -> &'static mut Reverb {
    // SAFETY: as `state`; only `update_buffers` uses it.
    unsafe { &mut *REVERB.0.get() }
}

// ---------------------------------------------------------------- entry points

const PARAM_FUZZ: i32 = 0;
const PARAM_REVERB: i32 = 1;
const PARAM_LEVEL: i32 = 2;

unsafe extern "C" fn patch_init(env: *const PatchEnv) {
    if env.is_null() || unsafe { (*env).abi_version } != PATCH_ABI_VERSION as u32 {
        return;
    }
    state().init();
}

unsafe extern "C" fn update_buffers(_: *const PatchEnv, left: *mut f32, right: *mut f32, samples: i32) -> f32 {
    let Ok(n) = usize::try_from(samples) else { return 0.0 };
    if left.is_null() || right.is_null() || n == 0 {
        return 0.0;
    }
    // SAFETY: the firmware passes two buffers of `samples` floats.
    let (l, r) = unsafe { (core::slice::from_raw_parts_mut(left, n), core::slice::from_raw_parts_mut(right, n)) };
    let s = state();
    if !s.ready {
        return 0.0;
    }
    let rev = reverb();
    for (a, b) in l.iter_mut().zip(r.iter_mut()) {
        let (ol, or) = s.tick(rev, *a);
        *a = ol;
        *b = or;
    }
    0.0
}

unsafe extern "C" fn set_buffer(_: *const PatchEnv, buffer: *mut f32) {
    // All state is static; the working buffer isn't needed.
    if buffer.is_null() {
        state().error = true;
    }
}

unsafe extern "C" fn get_buffer_size(_: *const PatchEnv) -> i32 {
    WORKING_BUFFER_SIZE
}

unsafe extern "C" fn get_param_min(_: *const PatchEnv, _: i32) -> f32 {
    0.0
}

unsafe extern "C" fn get_param_max(_: *const PatchEnv, _: i32) -> f32 {
    1.0
}

unsafe extern "C" fn get_param_default(_: *const PatchEnv, idx: i32) -> f32 {
    match idx {
        PARAM_FUZZ => 0.7,
        PARAM_REVERB => 0.35,
        _ => 0.75,
    }
}

unsafe extern "C" fn is_param_enabled(_: *const PatchEnv, idx: i32, source_id: i32) -> u8 {
    // Knobs only (source 0), like the SDK wrapper.
    (source_id == 0 && (0..PARAM_COUNT).contains(&idx)) as u8
}

unsafe fn write_c_str(s: &str, out: *mut u8, len: usize) {
    if out.is_null() || len == 0 {
        return;
    }
    let n = s.len().min(len - 1);
    // SAFETY: the firmware provides `len` writable bytes at `out`.
    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr(), out, n);
        *out.add(n) = 0;
    }
}

unsafe extern "C" fn get_param_name(_: *const PatchEnv, idx: i32, out: *mut u8, len: usize) {
    let name = match idx {
        PARAM_FUZZ => "Fuzz",
        PARAM_REVERB => "Reverb",
        PARAM_LEVEL => "Level",
        _ => "",
    };
    unsafe { write_c_str(name, out, len) }
}

unsafe extern "C" fn get_param_unit(_: *const PatchEnv, _: i32, out: *mut u8, len: usize) {
    unsafe { write_c_str("", out, len) }
}

unsafe extern "C" fn set_param(_: *const PatchEnv, idx: i32, value: f32) {
    let s = state();
    let v = value.clamp(0.0, 1.0);
    match idx {
        PARAM_FUZZ => s.fuzz = v,
        PARAM_REVERB => s.reverb = v,
        PARAM_LEVEL => s.level = v,
        _ => return,
    }
    if s.ready {
        s.retarget();
    }
}

unsafe extern "C" fn special_action(_: *const PatchEnv, idx: i32) {
    let s = state();
    match idx {
        ACTION_FOOTSWITCH_PRESS => s.fuzz_on = !s.fuzz_on,
        ACTION_FOOTSWITCH_HOLD => s.organ_on = !s.organ_on,
        _ => {}
    }
}

unsafe extern "C" fn get_state_idx(_: *const PatchEnv) -> i32 {
    state().led() as i32
}

// ---------------------------------------------------------------- header

#[cfg(target_os = "none")]
unsafe extern "C" {
    static __patch_bss_start: u8;
    static patch_image_size: u8;
    static patch_bss_size: u8;
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".patch_header")]
#[used]
pub static patch_header: PatchHeader = PatchHeader {
    magic: PATCH_MAGIC,
    abi_version: PATCH_ABI_VERSION,
    flags: PATCH_FLAG_NONE,
    init: patch_init,
    agent_update_buffers: update_buffers,
    agent_set_buffer: set_buffer,
    agent_get_buffer_size: get_buffer_size,
    agent_get_param_min: get_param_min,
    agent_get_param_max: get_param_max,
    agent_get_param_default: get_param_default,
    agent_is_param_enabled: is_param_enabled,
    agent_get_param_name: get_param_name,
    agent_get_param_unit: get_param_unit,
    agent_set_param: set_param,
    agent_special_action: special_action,
    agent_get_state_idx: get_state_idx,
    image_size: &raw const patch_image_size,
    bss_begin: &raw const __patch_bss_start,
    bss_size: &raw const patch_bss_size,
    reserved: [0; 16],
};

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_os = "none"))]
mod host;

#[cfg(not(target_os = "none"))]
fn main() {
    host::main();
}
