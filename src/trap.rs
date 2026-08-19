use core::arch::asm;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::ADAPTIVE_SCHED;
use crate::USE_AI;
use crate::{report_scheduler_error, puts, print_number};

const SBI_SET_TIMER: usize = 0;
const TIMEBASE_FREQ: u64 = 10_000_000; // 10 MHz (QEMU default)
const TIMER_INTERVAL: u64 = TIMEBASE_FREQ / 100; // 100 Hz (10ms)
const TIMER_INTERRUPT: usize = 7;

/// Сколько прерываний пришло с causeм, который мы не умеем обрабатывать.
static UNHANDLED_INTERRUPTS: AtomicUsize = AtomicUsize::new(0);

pub fn unhandled_interrupt_count() -> usize {
    UNHANDLED_INTERRUPTS.load(Ordering::Relaxed)
}

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

        let is_interrupt = mcause & (1 << 63) != 0;
        let cause = mcause & 0xFFF;

        if !is_interrupt {
            // Синхронное исключение (illegal instruction, page fault, ...).
            // Возврат из обработчика просто повторит сбойную инструкцию и
            // подвесит ядро без единого сообщения, поэтому останавливаемся здесь.
            let mepc: usize;
            let mtval: usize;
            asm!("csrr {}, mepc", out(reg) mepc, options(nostack));
            asm!("csrr {}, mtval", out(reg) mtval, options(nostack));

            puts("\n💥 FATAL: unhandled exception mcause=");
            print_number(cause);
            puts(" mepc=");
            print_number(mepc);
            puts(" mtval=");
            print_number(mtval);
            puts("\n");

            halt();
        }

        if cause != TIMER_INTERRUPT {
            UNHANDLED_INTERRUPTS.fetch_add(1, Ordering::Relaxed);
            puts("\n⚠️  Unhandled interrupt, mcause=");
            print_number(cause);
            puts("\n");
            return;
        }

        // AI scheduler
        if USE_AI.load(Ordering::Relaxed) == 1 {
            match ADAPTIVE_SCHED {
                Some(ref mut adaptive) => {
                    if let Err(error) = adaptive.adaptive_schedule() {
                        report_scheduler_error("adaptive_schedule", error);
                    }
                }
                None => {
                    puts("\n⚠️  AI scheduling enabled but scheduler is not initialized\n");
                }
            }
        }

        let next = read_time() + TIMER_INTERVAL;
        sbi_set_timer(next);
    }
}

fn halt() -> ! {
    loop {
        unsafe {
            asm!("wfi", options(nostack));
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
