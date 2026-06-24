use core::arch::asm;
use crate::ADAPTIVE_SCHED;
use crate::USE_AI;

const SBI_SET_TIMER: usize = 0;
const TIMEBASE_FREQ: u64 = 10_000_000; // 10 MHz (QEMU default)
const TIMER_INTERVAL: u64 = TIMEBASE_FREQ / 100; // 100 Hz (10ms)

/// SBI call to set timer
unsafe fn sbi_set_timer(stime: u64) {
    asm!(
        "li a7, 0",           // a7 = 0 (SBI_SET_TIMER)
        "mv a0, {}",          // a0 = stime
        "ecall",              // SBI call
        in(reg) stime,
        options(nostack)
    );
}

/// Read current time from mcycle
unsafe fn read_time() -> u64 {
    let time: u64;
    asm!(
        "rdcycle {}",
        out(reg) time,
        options(nostack)
    );
    time
}

/// Main trap handler called from assembly
#[no_mangle]
pub extern "C" fn rust_trap_handler() {
    unsafe {
        let mcause: usize;
        asm!("csrr {}, mcause", out(reg) mcause, options(nostack));

        // Check if this is an interrupt (bit 63 set)
        if mcause & (1 << 63) != 0 {
            let cause = mcause & 0xFFF;

            // Only handle timer interrupts (cause = 7)
            if cause == 7 {
                // Используем AI планировщик если он инициализирован
                if USE_AI.load(core::sync::atomic::Ordering::Relaxed) == 1 {
                    if let Some(ref mut adaptive) = ADAPTIVE_SCHED {
                        adaptive.adaptive_schedule();
                    }
                }

                // Set next timer
                let next = read_time() + TIMER_INTERVAL;
                sbi_set_timer(next);
            }
        }
    }
}

/// Initialize the timer and interrupts
pub unsafe fn init_timer() {
    // 1. Disable all interrupts
    asm!("csrw mie, 0", options(nostack));

    // 2. Enable machine timer interrupt (bit 7)
    asm!("csrrsi t0, mie, 7", options(nostack));

    // 3. Enable global interrupts (MIE bit in mstatus)
    asm!("csrrsi t0, mstatus, 8", options(nostack));

    // 4. Set first timer
    let next = read_time() + TIMER_INTERVAL;
    sbi_set_timer(next);
}
