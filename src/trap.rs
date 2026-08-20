use core::arch::asm;
use core::sync::atomic::Ordering;
use crate::trapframe::TrapFrame;
use crate::ADAPTIVE_SCHED;
use crate::USE_AI;

// QEMU `virt` CLINT (Core Local Interruptor) registers for hart 0.
const CLINT_MTIME: *mut u64 = 0x0200_bff8 as *mut u64;
const CLINT_MTIMECMP: *mut u64 = 0x0200_4000 as *mut u64;

// mtime runs at 10 MHz on QEMU virt; time slice between preemptions.
const TIMER_INTERVAL: u64 = 100_000; // ~10 ms quantum

// Print a statistics block every N timer ticks.
const STATS_EVERY: usize = 40;

/// Pointer to the TrapFrame of the currently running task. Read and written by
/// the assembly trap vector (trap.s) and updated here when the scheduler picks
/// a new task.
#[no_mangle]
pub static mut CURRENT_TF: *mut TrapFrame = core::ptr::null_mut();

static mut TICKS: usize = 0;

extern "C" {
    fn trap_vector();
    /// Launch the first task (never returns). Defined in trap.s.
    pub fn start_scheduling() -> !;
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
                    // Neural / fairness / Q-learning picks the next task.
                    adaptive.adaptive_schedule();

                    // Point the trap vector at the newly selected task's frame.
                    let cur = adaptive.base_scheduler.current;
                    if let Some(ref mut task) = adaptive.base_scheduler.tasks[cur] {
                        CURRENT_TF = &mut task.tf as *mut TrapFrame;
                    }

                    TICKS += 1;
                    if TICKS % STATS_EVERY == 0 {
                        adaptive.collect_statistics();
                        crate::print_stats(adaptive);
                    }
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
    // Global interrupt enable (mstatus.MIE) is set from each task's saved
    // mstatus by mret, so no need to set it here.
}
