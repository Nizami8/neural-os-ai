use crate::config::{CLINT_MTIME, CLINT_MTIMECMP, TIMER_INTERVAL};

#[cfg(not(any(test, feature = "std")))]
pub fn read_mtime() -> u64 {
    unsafe { core::ptr::read_volatile(CLINT_MTIME as *const u64) }
}

#[cfg(not(any(test, feature = "std")))]
pub fn set_mtimecmp(value: u64) {
    unsafe { core::ptr::write_volatile(CLINT_MTIMECMP as *mut u64, value) }
}

#[cfg(not(any(test, feature = "std")))]
pub fn init() {
    set_mtimecmp(read_mtime().wrapping_add(TIMER_INTERVAL));
    unsafe {
        core::arch::asm!(
            "li t0, 128",
            "csrs mie, t0",
            "csrsi mstatus, 8",
            options(nostack)
        );
    }
}

#[cfg(not(any(test, feature = "std")))]
pub fn ack() {
    set_mtimecmp(read_mtime().wrapping_add(TIMER_INTERVAL));
}

#[cfg(any(test, feature = "std"))]
pub fn init() {}

#[cfg(any(test, feature = "std"))]
pub fn ack() {}
