//! Minimal system-call ABI (Stage 3).
//!
//! Tasks trap into the kernel with `ecall`; the machine-mode trap handler
//! (`src/trap.rs`) decodes `a7` as the syscall number, `a0`/`a1` as arguments,
//! and returns a result in `a0`. Because the trap vector saves and restores the
//! entire register file, an `ecall` preserves all task registers except the
//! documented return value.

use core::arch::asm;

pub const SYS_YIELD: usize = 0;
pub const SYS_GETPID: usize = 1;
pub const SYS_PRINT: usize = 2;
pub const SYS_EXIT: usize = 3;
pub const SYS_SEND: usize = 4;
pub const SYS_RECV: usize = 5;
pub const SYS_PS: usize = 6;
pub const SYS_STATS: usize = 7;
pub const SYS_NN: usize = 8;

/// Return the current task's id.
pub fn sys_getpid() -> usize {
    let ret: usize;
    unsafe {
        asm!("ecall", in("a7") SYS_GETPID, lateout("a0") ret);
    }
    ret
}

/// Print a string to the console via the kernel.
pub fn sys_print(s: &str) {
    let ptr = s.as_ptr() as usize;
    let len = s.len();
    unsafe {
        asm!(
            "ecall",
            in("a7") SYS_PRINT,
            inout("a0") ptr => _,
            in("a1") len,
        );
    }
}

/// Print an unsigned integer via `SYS_PRINT`.
pub fn sys_print_usize(mut n: usize) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    if n == 0 {
        i -= 1;
        buf[i] = b'0';
    }
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    if let Ok(s) = core::str::from_utf8(&buf[i..]) {
        sys_print(s);
    }
}

/// Voluntarily yield the CPU to the scheduler.
pub fn sys_yield() {
    unsafe {
        asm!("ecall", in("a7") SYS_YIELD);
    }
}

/// Terminate the current task. Never returns (the scheduler never selects a
/// terminated task again).
pub fn sys_exit() -> ! {
    unsafe {
        asm!("ecall", in("a7") SYS_EXIT);
    }
    loop {}
}

/// Send a one-word message on an endpoint (blocks until a receiver takes it).
/// Returns 0 on success, or a non-zero error code (e.g. EPERM if the task lacks
/// the SEND capability on this endpoint).
pub fn sys_send(endpoint: usize, msg: usize) -> usize {
    let rc: usize;
    unsafe {
        asm!(
            "ecall",
            in("a7") SYS_SEND,
            inout("a0") endpoint => rc,
            in("a1") msg,
        );
    }
    rc
}

/// Receive a one-word message from an endpoint (blocks until a sender arrives).
pub fn sys_recv(endpoint: usize) -> usize {
    let msg: usize;
    unsafe {
        asm!(
            "ecall",
            in("a7") SYS_RECV,
            inout("a0") endpoint => msg,
        );
    }
    msg
}

/// Ask the kernel to print the task table.
pub fn sys_ps() {
    unsafe {
        asm!("ecall", in("a7") SYS_PS);
    }
}

/// Ask the kernel to print scheduler statistics.
pub fn sys_stats() {
    unsafe {
        asm!("ecall", in("a7") SYS_STATS);
    }
}

/// Ask the kernel to print the neural network's output weights.
pub fn sys_nn() {
    unsafe {
        asm!("ecall", in("a7") SYS_NN);
    }
}
