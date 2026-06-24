use core::arch::asm;
use crate::ADAPTIVE_SCHED;
use crate::USE_AI;

const SBI_SET_TIMER: usize = 0;
const TIMEBASE_FREQ: u64 = 10_000_000; // 10 MHz (QEMU default)
const TIMER_INTERVAL: u64 = TIMEBASE_FREQ / 100; // 100 Hz (10ms)

unsafe fn sbi_set_timer(stime: u64) {
    asm!(
        "li a7, 0",
        "mv a0, {}",
        "ecall",
        in(reg) stime,
        options(nostack)
    );
}

unsafe fn read_time() -> u64 {
    let time: u64;
    asm!(
        "rdcycle {}",
        out(reg) time,
        options(nostack)
    );
    time
}

#[no_mangle]
pub extern "C" fn rust_trap_handler() {
    unsafe {
        let mcause: usize;
        asm!("csrr {}, mcause", out(reg) mcause, options(nostack));

        if mcause & (1 << 63) != 0 {
            let cause = mcause & 0xFFF;

            if cause == 7 {
                // AI scheduler
                if USE_AI.load(core::sync::atomic::Ordering::Relaxed) == 1 {
                    if let Some(ref mut adaptive) = ADAPTIVE_SCHED {
                        adaptive.adaptive_schedule();
                    }
                }

                let next = read_time() + TIMER_INTERVAL;
                sbi_set_timer(next);
            }
        }
    }
}

pub unsafe fn init_timer() {
    asm!("csrw mie, 0", options(nostack));
    asm!("csrrsi t0, mie, 7", options(nostack));
    asm!("csrrsi t0, mstatus, 8", options(nostack));

    let next = read_time() + TIMER_INTERVAL;
    sbi_set_timer(next);
}
