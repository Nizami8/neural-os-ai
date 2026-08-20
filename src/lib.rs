#![cfg_attr(not(any(test, feature = "std")), no_std)]

pub mod adaptive;
pub mod capability;
pub mod config;
pub mod console;
pub mod context;
pub mod cpu;
pub mod ipc;
pub mod kernel;
pub mod neural;
pub mod process;
pub mod scheduler;
pub mod storage;
pub mod syscall;
pub mod thread;
pub mod timer;
pub mod trap;
pub mod trapframe;
pub mod uart;

#[cfg(not(any(test, feature = "std")))]
core::arch::global_asm!(include_str!("start.S"));

#[cfg(not(any(test, feature = "std")))]
core::arch::global_asm!(include_str!("trap.S"));

#[cfg(not(any(test, feature = "std")))]
core::arch::global_asm!(include_str!("context.S"));

#[cfg(not(any(test, feature = "std")))]
#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[cfg(not(any(test, feature = "std")))]
#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.add(i) = c as u8;
        i += 1;
    }
    s
}

#[cfg(not(any(test, feature = "std")))]
#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let d = *a.add(i) as i32 - *b.add(i) as i32;
        if d != 0 {
            return d;
        }
        i += 1;
    }
    0
}
