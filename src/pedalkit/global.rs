//! A static cell for patch state shared with the firmware's entry points.

use core::cell::UnsafeCell;

/// The firmware calls every entry point from the audio thread (SDK: "no need
/// to synchronize"), so a plain cell is enough.
pub struct Global<T>(UnsafeCell<T>);

unsafe impl<T> Sync for Global<T> {}

impl<T> Global<T> {
    pub const fn new(v: T) -> Self {
        Self(UnsafeCell::new(v))
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get(&self) -> &mut T {
        // SAFETY: single-threaded access from the firmware's audio thread.
        unsafe { &mut *self.0.get() }
    }
}
