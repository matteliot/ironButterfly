//! Delay memory. A pedal takes every buffer it needs from a `Pool` in `init`,
//! sized exactly: a standalone patch backs the pool with a static array in
//! `.bss`, superPedal with the firmware's working buffer (shared by all its
//! pedals, since only one plays at a time).

use super::global::Global;

/// Where a buffer points when the pool ran out: harmless garbage instead of a
/// fault. The runner shows the overflow on the LED.
static SCRATCH: Global<[f32; 4]> = Global::new([0.0; 4]);

pub struct Pool {
    ptr: *mut f32,
    cap: usize,
    used: usize,
    overflow: bool,
}

impl Pool {
    pub const EMPTY: Self = Self { ptr: core::ptr::null_mut(), cap: 0, used: 0, overflow: false };

    /// # Safety
    /// `ptr` must point to `cap` floats that nothing else uses while the pool
    /// and the buffers taken from it live.
    pub const unsafe fn new(ptr: *mut f32, cap: usize) -> Self {
        Self { ptr, cap, used: 0, overflow: false }
    }

    /// `n` floats (not cleared: `clear` the pool before use), or `None` if
    /// the pool is out of memory (then flagged as overflowed).
    pub fn take(&mut self, n: usize) -> Option<*mut f32> {
        if self.ptr.is_null() || self.used + n > self.cap {
            self.overflow = true;
            return None;
        }
        // SAFETY: in bounds, checked above.
        let p = unsafe { self.ptr.add(self.used) };
        self.used += n;
        Some(p)
    }

    /// The 4-float scratch a buffer falls back to when the pool is out of memory.
    fn scratch() -> *mut f32 {
        SCRATCH.get().as_mut_ptr()
    }

    pub fn used(&self) -> usize {
        self.used
    }

    pub fn overflowed(&self) -> bool {
        self.overflow
    }

    /// Forgets every buffer taken, so the memory can be handed out again.
    pub fn reset(&mut self) {
        self.used = 0;
        self.overflow = false;
    }

    /// Zeroes floats `from..to` (clamped to the pool): a NaN left in a
    /// feedback loop never decays. Done in place, in chunks if the caller wants.
    pub fn clear(&mut self, from: usize, to: usize) {
        let to = to.min(self.cap);
        if self.ptr.is_null() || from >= to {
            return;
        }
        // SAFETY: in bounds of the pool's memory.
        unsafe { core::ptr::write_bytes(self.ptr.add(from), 0, to - from) }
    }
}

/// A delay line of a power-of-two length with fractional reads.
pub struct Line {
    p: *mut f32,
    mask: usize,
    w: usize,
}

impl Line {
    pub const NEW: Self = Self { p: core::ptr::null_mut(), mask: 0, w: 0 };

    /// Takes `n` floats from the pool; `n` must be a power of two.
    pub fn take(&mut self, pool: &mut Pool, n: usize) {
        debug_assert!(n.is_power_of_two());
        (self.p, self.mask) = match pool.take(n) {
            Some(p) => (p, n - 1),
            None => (Pool::scratch(), 3),
        };
        self.w = 0;
    }

    #[inline]
    pub fn push(&mut self, x: f32) {
        self.w = (self.w + 1) & self.mask;
        // SAFETY: masked into the buffer.
        unsafe { *self.p.add(self.w) = x }
    }

    /// Silences the line.
    pub fn clear(&mut self) {
        if !self.p.is_null() {
            // SAFETY: mask + 1 floats were taken (or the 4-float scratch).
            unsafe { core::ptr::write_bytes(self.p, 0, self.mask + 1) }
        }
        self.w = 0;
    }

    /// The sample `d` samples ago (linear interpolation; d = 0 is the newest).
    #[inline]
    pub fn read(&self, d: f32) -> f32 {
        let d = d.clamp(0.0, (self.mask - 1) as f32);
        let i = d as usize;
        let fr = d - i as f32;
        // SAFETY: masked into the buffer.
        let (a, b) = unsafe {
            (*self.p.add(self.w.wrapping_sub(i) & self.mask), *self.p.add(self.w.wrapping_sub(i + 1) & self.mask))
        };
        a + (b - a) * fr
    }

    /// The sample exactly `i` samples ago.
    #[inline]
    pub fn at(&self, i: usize) -> f32 {
        // SAFETY: masked into the buffer.
        unsafe { *self.p.add(self.w.wrapping_sub(i) & self.mask) }
    }
}

/// A circular buffer of any length, stepped one slot per sample (comb and
/// allpass filters, block windows).
pub struct Ring {
    p: *mut f32,
    len: usize,
    i: usize,
}

impl Ring {
    pub const NEW: Self = Self { p: core::ptr::null_mut(), len: 0, i: 0 };

    pub fn take(&mut self, pool: &mut Pool, n: usize) {
        (self.p, self.len) = match pool.take(n) {
            Some(p) => (p, n),
            None => (Pool::scratch(), 4),
        };
        self.i = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// The slot under the cursor.
    #[inline]
    pub fn get(&self) -> f32 {
        // SAFETY: i < len.
        unsafe { *self.p.add(self.i) }
    }

    #[inline]
    pub fn set(&mut self, x: f32) {
        // SAFETY: i < len.
        unsafe { *self.p.add(self.i) = x }
    }

    /// Moves the cursor on one slot, wrapping at the end.
    #[inline]
    pub fn step(&mut self) {
        self.i += 1;
        if self.i >= self.len {
            self.i = 0;
        }
    }

    /// The whole buffer, for block processing.
    #[inline]
    pub fn slice(&mut self) -> &mut [f32] {
        // SAFETY: `len` floats taken from the pool (or the scratch).
        unsafe { core::slice::from_raw_parts_mut(self.p, self.len) }
    }

    /// Slot `k` (any index, wrapped), for block processing.
    #[inline]
    pub fn slot(&mut self, k: usize) -> &mut f32 {
        // SAFETY: wrapped into the buffer.
        unsafe { &mut *self.p.add(k % self.len) }
    }
}
