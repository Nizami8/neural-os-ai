//! Interrupt-safe interior mutability for kernel globals.
//!
//! Bare-metal traps run on a single hart with interrupts masked inside the
//! handler, so exclusive `&mut` access through these cells is sound as long as
//! callers never re-enter the same cell while a borrow is live.

use core::cell::UnsafeCell;

/// Transparent wrapper so `#[no_mangle] static` symbols keep a plain layout
/// (required for assembly that loads `CURRENT_TF` by address).
#[repr(transparent)]
pub struct KernelCell<T>(UnsafeCell<T>);

// SAFETY: single-hart kernel; exclusive access is enforced by interrupt masking
// in the trap path and by not holding references across await points.
unsafe impl<T> Sync for KernelCell<T> {}

impl<T> KernelCell<T> {
    pub const fn new(value: T) -> Self {
        KernelCell(UnsafeCell::new(value))
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.0.get()
    }

    /// # Safety
    /// Caller must ensure exclusive access for the lifetime of the reference.
    #[inline]
    pub unsafe fn as_ref(&self) -> &T {
        &*self.0.get()
    }

    /// # Safety
    /// Caller must ensure exclusive access for the lifetime of the reference.
    #[inline]
    pub unsafe fn as_mut(&self) -> &mut T {
        &mut *self.0.get()
    }
}
