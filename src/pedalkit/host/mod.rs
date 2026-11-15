//! Host-only demo rendering, shared by every pedal's `cargo sim`: plays a
//! score on the modelled guitar, drives the patch's entry points the way the
//! firmware does (128-sample blocks, footswitch and knob events at set
//! times), prints per-section stats and writes a stereo float WAV.

pub mod stats;
pub mod wav;

use super::abi::*;
use super::guitar::{Note, Player};

pub const BLOCK: usize = 128;
pub const SRU: usize = 48_000;

/// A patch's entry points (`endless_patch!` makes this as `entry::ENTRY`).
pub struct Entry {
    pub patch_init: unsafe extern "C" fn(*const PatchEnv),
    pub update_buffers: unsafe extern "C" fn(*const PatchEnv, *mut f32, *mut f32, i32) -> f32,
    pub set_buffer: unsafe extern "C" fn(*const PatchEnv, *mut f32),
    pub get_param_default: unsafe extern "C" fn(*const PatchEnv, i32) -> f32,
    pub set_param: unsafe extern "C" fn(*const PatchEnv, i32, f32),
    pub special_action: unsafe extern "C" fn(*const PatchEnv, i32),
    pub get_state_idx: unsafe extern "C" fn(*const PatchEnv) -> i32,
    pub get_param_name: unsafe extern "C" fn(*const PatchEnv, i32, *mut u8, usize),
    pub pool_used: fn() -> usize,
}

/// Something that happens at a point in the score.
pub enum Ev {
    /// Play these notes, strummed this many seconds apart; `true` lets what's ringing carry on.
    Play(Vec<Note>, f32, bool),
    /// The fretting hand lets go of everything.
    Release,
    Press,
    Hold,
    /// Turn knob `idx` to a value.
    Knob(i32, f32),
    /// A pedal-specific event for the demo's `Extra` (id, value).
    Custom(u32, f32),
}

/// Pedal-specific additions to a demo render.
pub trait Extra {
    /// An `Ev::Custom` event came up.
    fn event(&mut self, _id: u32, _v: f32) {}
    /// Whether `tick` adds a signal to the guitar.
    fn has_source(&self) -> bool {
        false
    }
    /// One sample of an extra input signal, added to the guitar (before the input trim).
    fn tick(&mut self) -> f32 {
        0.0
    }
    /// Called after every block (to watch the pedal's state).
    fn block(&mut self) {}
}

/// No additions.
pub struct NoExtra;
impl Extra for NoExtra {}

/// A per-block callback as an `Extra`.
struct Hook<'a>(&'a mut dyn FnMut());
impl Extra for Hook<'_> {
    fn block(&mut self) {
        (self.0)()
    }
}

/// (time s, event, section label: non-empty starts a new stats section).
pub type Score = Vec<(f64, Ev, &'static str)>;

pub struct Render {
    pub l: Vec<f32>,
    pub r: Vec<f32>,
    /// The input fed to the patch (left channel, after the trim).
    pub input: Vec<f32>,
    /// (start sample, label)
    pub sections: Vec<(usize, &'static str)>,
    pub env: PatchEnv,
    /// Kept alive: the patch may hold on to it.
    pub work: Vec<f32>,
}

impl Render {
    /// The samples (left, right) of the first section whose label starts with `prefix`.
    pub fn section(&self, prefix: &str) -> Option<(&[f32], &[f32])> {
        let i = self.sections.iter().position(|(_, l)| l.starts_with(prefix))?;
        let a = self.sections[i].0;
        let b = self.sections.get(i + 1).map_or(self.l.len(), |s| s.0);
        Some((&self.l[a..b], &self.r[a..b]))
    }
}

pub struct Options {
    /// The first section's label.
    pub first: &'static str,
    /// The guitar into the patch, matched to the level the patch was voiced for.
    pub input_trim: f32,
    /// Floats in the working buffer handed to `set_buffer` (16 when unused).
    pub working: usize,
}

/// Plays `score` (lasting `total_s`) through the patch.
pub fn render(e: &Entry, events: Score, total_s: f64, o: Options) -> Render {
    render_extra(e, events, total_s, o, &mut NoExtra)
}

/// As `render`, calling `hook` after every block (to watch the pedal's state).
pub fn render_with(e: &Entry, events: Score, total_s: f64, o: Options, hook: &mut dyn FnMut()) -> Render {
    render_extra(e, events, total_s, o, &mut Hook(hook))
}

/// As `render`, with pedal-specific additions.
pub fn render_extra(e: &Entry, mut events: Score, total_s: f64, o: Options, extra: &mut dyn Extra) -> Render {
    let env = PatchEnv { abi_version: PATCH_ABI_VERSION as u32, user_ctx: core::ptr::null_mut() };
    let mut work = vec![0.0f32; o.working];
    unsafe {
        (e.patch_init)(&env);
        (e.set_buffer)(&env, work.as_mut_ptr());
        for i in 0..PARAM_COUNT {
            (e.set_param)(&env, i, (e.get_param_default)(&env, i));
        }
    }
    events.sort_by(|a, b| b.0.total_cmp(&a.0));
    let total = (total_s * SRU as f64) as usize;
    let mut guitar = Player::new();
    let mut lo = vec![];
    let mut ro = vec![];
    let mut input = vec![];
    let mut sections: Vec<(usize, &str)> = vec![(0, o.first)];
    // PEDAL_DUMP=<dir>: also write the input and the timed events (to replay
    // through another implementation of the same pedal).
    let mut log = String::new();
    let mut t = 0;
    while t < total {
        while events.last().is_some_and(|(et, _, _)| (*et * SRU as f64) as usize <= t) {
            let (_, ev, label) = events.pop().unwrap();
            match ev {
                Ev::Press => {
                    log += &format!("{t} P\n");
                    unsafe { (e.special_action)(&env, ACTION_FOOTSWITCH_PRESS) }
                }
                Ev::Hold => {
                    log += &format!("{t} H\n");
                    unsafe { (e.special_action)(&env, ACTION_FOOTSWITCH_HOLD) }
                }
                Ev::Knob(i, v) => {
                    log += &format!("{t} K {i} {v}\n");
                    unsafe { (e.set_param)(&env, i, v) }
                }
                Ev::Release => guitar.release_all(),
                Ev::Custom(id, v) => extra.event(id, v),
                Ev::Play(notes, strum, ring) => guitar.play(&notes, strum, ring),
            }
            if !label.is_empty() {
                sections.push((t, label));
            }
        }
        let mut l = [0.0f32; BLOCK];
        let mut r = [0.0f32; BLOCK];
        if extra.has_source() {
            for x in l.iter_mut() {
                *x = (guitar.tick() + extra.tick()) * o.input_trim;
            }
        } else {
            for x in l.iter_mut() {
                *x = guitar.tick() * o.input_trim;
            }
        }
        input.extend_from_slice(&l);
        unsafe { (e.update_buffers)(&env, l.as_mut_ptr(), r.as_mut_ptr(), BLOCK as i32) };
        extra.block();
        lo.extend_from_slice(&l);
        ro.extend_from_slice(&r);
        t += BLOCK;
    }
    if let Some(dir) = std::env::var_os("PEDAL_DUMP") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        let bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();
        std::fs::write(dir.join("input.f32"), bytes).unwrap();
        std::fs::write(dir.join("events.txt"), log).unwrap();
    }
    Render { l: lo, r: ro, input, sections, env, work }
}

/// Prints the stats and the LED, writes the WAV to `out`, and exits with an
/// error on NaN or full-scale output.
pub fn finish(e: &Entry, r: Render, out: &str) {
    let ok = stats::report(&r.l, &r.r, r.sections.clone());
    println!("final LED colour index: {}", unsafe { (e.get_state_idx)(&r.env) });
    println!("delay memory used: {} floats", (e.pool_used)());
    if let Some(dir) = std::path::Path::new(out).parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    wav::write_wav(out, &r.l, &r.r);
    println!("wrote {out} ({:.1} s)", r.l.len() as f64 / SRU as f64);
    if !ok {
        eprintln!("error: NaN or full-scale output");
        std::process::exit(1);
    }
}

/// The output path: the first command-line argument, or `default`.
pub fn out_path(default: &str) -> String {
    std::env::args().nth(1).unwrap_or(default.into())
}
