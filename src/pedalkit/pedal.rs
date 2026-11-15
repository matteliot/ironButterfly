//! What every pedal is to the firmware, and the generic runner that the
//! entry points (`endless_patch!`) drive.

use super::abi::Color;
use super::pool::Pool;

/// One knob: the name the patch reports to the firmware, and its default (0..1).
pub struct Param {
    pub name: &'static str,
    pub default: f32,
}

/// A complete pedal: three knobs, a footswitch (press and hold), stereo
/// audio and an LED. superPedal is one too.
pub trait Pedal {
    /// The pedal with everything at rest (a `static`'s initial value).
    const NEW: Self;
    /// The knobs, left to right.
    const PARAMS: [Param; 3];
    /// Delay memory (floats) a standalone build reserves in `.bss`.
    const POOL: usize;

    /// Sets coefficients and takes its delay memory. The runner clears the
    /// pool afterwards, so buffers start silent.
    fn init(&mut self, pool: &mut Pool);
    /// Knob `idx` (0..3) moved to `v` (0..1). May be called before `init`.
    fn set_knob(&mut self, idx: usize, v: f32);
    fn press(&mut self);
    fn hold(&mut self);
    /// One sample in, one stereo sample out.
    fn tick(&mut self, l: f32, r: f32) -> (f32, f32);
    fn led(&self) -> Color;

    /// The firmware's working buffer arrived (superPedal uses it for its pool).
    fn attach(&mut self, _ptr: *mut f32, _len: usize) {}
    /// Called once per audio block before the samples (block-rate work).
    fn block(&mut self) {}
}

/// Where a standalone patch's delay memory lives.
pub enum Memory {
    /// A static `.bss` array (pointer and length), zeroed in `patch_init`.
    Static(*mut f32, usize),
    /// None up front: the pedal takes the firmware's working buffer in `attach`.
    Working,
}

/// Drives a `Pedal` from the firmware's entry points.
pub struct Runner<P: Pedal> {
    pedal: P,
    pool: Pool,
    ready: bool,
    /// The firmware didn't provide the working buffer, or the pool ran out.
    error: bool,
}

impl<P: Pedal> Runner<P> {
    pub const NEW: Self = Self { pedal: P::NEW, pool: Pool::EMPTY, ready: false, error: false };

    pub fn init(&mut self, mem: Memory) {
        if let Memory::Static(ptr, len) = mem {
            // SAFETY: the caller hands over its static array.
            self.pool = unsafe { Pool::new(ptr, len) };
        }
        self.pedal.init(&mut self.pool);
        self.pool.clear(0, self.pool.used());
        if self.pool.overflowed() {
            self.error = true;
        }
        self.ready = true;
    }

    pub fn set_buffer(&mut self, ptr: *mut f32, len: usize) {
        if ptr.is_null() {
            self.error = true;
        } else {
            self.pedal.attach(ptr, len);
        }
    }

    pub fn update(&mut self, l: &mut [f32], r: &mut [f32]) {
        if !self.ready {
            return;
        }
        self.pedal.block();
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            let (ol, or) = self.pedal.tick(*a, *b);
            *a = ol;
            *b = or;
        }
    }

    pub fn set_param(&mut self, idx: i32, v: f32) {
        if let Ok(i) = usize::try_from(idx) {
            if i < 3 {
                self.pedal.set_knob(i, v.clamp(0.0, 1.0));
            }
        }
    }

    pub fn action(&mut self, idx: i32) {
        match idx {
            super::abi::ACTION_FOOTSWITCH_PRESS => self.pedal.press(),
            super::abi::ACTION_FOOTSWITCH_HOLD => self.pedal.hold(),
            _ => {}
        }
    }

    pub fn led(&self) -> Color {
        if self.error { Color::Red } else { self.pedal.led() }
    }

    /// The pedal (host-side analysis only).
    pub fn pedal(&mut self) -> &mut P {
        &mut self.pedal
    }

    /// Delay memory taken so far (floats), for the host's budget check.
    pub fn pool_used(&self) -> usize {
        self.pool.used()
    }

    /// Knob `idx`'s name and default.
    pub fn param(idx: i32) -> Option<(&'static str, f32)> {
        let i = usize::try_from(idx).ok().filter(|i| *i < 3)?;
        let p = &P::PARAMS[i];
        Some((p.name, p.default))
    }
}

/// Writes `s` as a NUL-terminated C string into `out` (`len` bytes).
///
/// # Safety
/// `out` must be writable for `len` bytes.
pub unsafe fn write_c_str(s: &str, out: *mut u8, len: usize) {
    if out.is_null() || len == 0 {
        return;
    }
    let n = s.len().min(len - 1);
    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr(), out, n);
        *out.add(n) = 0;
    }
}
