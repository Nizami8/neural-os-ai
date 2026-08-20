#[cfg(target_os = "none")]
use crate::config::{CLINT_MTIME, CLINT_MTIMECMP, TIMER_INTERVAL};

#[cfg(target_os = "none")]
pub fn read_mtime() -> u64 {
    unsafe { core::ptr::read_volatile(CLINT_MTIME as *const u64) }
}

#[cfg(target_os = "none")]
pub fn set_mtimecmp(value: u64) {
    unsafe { core::ptr::write_volatile(CLINT_MTIMECMP as *mut u64, value) }
}

#[cfg(target_os = "none")]
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

#[cfg(target_os = "none")]
pub fn ack() {
    set_mtimecmp(read_mtime().wrapping_add(TIMER_INTERVAL));
}

#[cfg(not(target_os = "none"))]
pub fn init() {}

#[cfg(not(target_os = "none"))]
pub fn ack() {}
