use core::arch::asm;
use core::sync::atomic::Ordering;
use crate::ADAPTIVE_SCHED;
use crate::USE_AI;

// QEMU `virt` CLINT (Core Local Interruptor) registers for hart 0.
const CLINT_MTIME: *mut u64 = 0x0200_bff8 as *mut u64;
const CLINT_MTIMECMP: *mut u64 = 0x0200_4000 as *mut u64;

// mtime runs at 10 MHz on QEMU virt; ~2 ms per scheduler tick.
const TIMER_INTERVAL: u64 = 20_000;

extern "C" {
    // Assembly trap vector (trap.s) that saves state and calls rust_trap_handler.
    fn trap_vector();
}

unsafe fn read_mtime() -> u64 {
    core::ptr::read_volatile(CLINT_MTIME)
}

unsafe fn schedule_next_timer() {
    let now = read_mtime();
    core::ptr::write_volatile(CLINT_MTIMECMP, now.wrapping_add(TIMER_INTERVAL));
}

/// Called from the assembly trap vector on every machine trap.
#[no_mangle]
pub extern "C" fn rust_trap_handler() {
    unsafe {
        let mcause: usize;
        asm!("csrr {}, mcause", out(reg) mcause, options(nostack));

        // Bit 63 set => interrupt; cause 7 => machine timer interrupt.
        if (mcause >> 63) != 0 && (mcause & 0xFFF) == 7 {
            if USE_AI.load(Ordering::Relaxed) == 1 {
                if let Some(ref mut adaptive) = ADAPTIVE_SCHED {
                    adaptive.adaptive_schedule();
                }
            }
            schedule_next_timer();
        }
    }
}

/// Configure machine-mode timer interrupts via the CLINT.
pub unsafe fn init_timer() {
    // Point mtvec at the assembly vector (direct mode: low bits = 0).
    asm!("csrw mtvec, {}", in(reg) trap_vector as usize, options(nostack));

    // Arm the first timer compare.
    schedule_next_timer();

    // Enable machine timer interrupt (MTIE = bit 7).
    asm!("csrs mie, {}", in(reg) 1usize << 7, options(nostack));
    // Enable global machine interrupts (MIE = bit 3).
    asm!("csrs mstatus, {}", in(reg) 1usize << 3, options(nostack));
}
