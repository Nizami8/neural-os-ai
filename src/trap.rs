use core::arch::asm;
use core::sync::atomic::Ordering;
use crate::trapframe::TrapFrame;
use crate::scheduler::{TaskState, MAX_TASKS};
use crate::syscall;
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

        let is_interrupt = (mcause >> 63) != 0;
        let code = mcause & 0xFFF;

        if is_interrupt {
            // cause 7 => machine timer interrupt.
            if code == 7 {
                on_timer_tick();
                schedule_next_timer();
            }
        } else {
            // cause 11 => environment call (ecall) from M-mode.
            if code == 11 {
                handle_syscall();
            }
        }
    }
}

unsafe fn on_timer_tick() {
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
}

/// Decode and service an `ecall` from the running task. Uses the saved
/// TrapFrame (CURRENT_TF) for arguments/return value and advances mepc past the
/// ecall so the task resumes at the following instruction.
unsafe fn handle_syscall() {
    let tf = CURRENT_TF; // raw pointer; avoid aliasing &mut with the scheduler
    if tf.is_null() {
        return;
    }
    let num = (*tf).regs[17]; // a7
    let arg0 = (*tf).regs[10]; // a0
    let arg1 = (*tf).regs[11]; // a1

    // Resume after the ecall for whichever task made the call.
    (*tf).mepc = (*tf).mepc.wrapping_add(4);

    match num {
        syscall::SYS_GETPID => {
            let pid = ADAPTIVE_SCHED
                .as_ref()
                .and_then(|a| {
                    let cur = a.base_scheduler.current;
                    a.base_scheduler.tasks[cur].as_ref().map(|t| t.id)
                })
                .unwrap_or(0);
            (*tf).regs[10] = pid; // return value in a0
        }
        syscall::SYS_PRINT => {
            let ptr = arg0 as *const u8;
            let mut i = 0;
            while i < arg1 {
                crate::putc(*ptr.add(i));
                i += 1;
            }
        }
        syscall::SYS_YIELD => switch_away(false),
        syscall::SYS_EXIT => switch_away(true),
        _ => {}
    }
}

/// Switch the CPU away from the current task. If `terminate`, the current task
/// is marked terminated and will never be scheduled again; otherwise it stays
/// runnable (a yield). Picks the next Ready task round-robin.
unsafe fn switch_away(terminate: bool) {
    let a = match ADAPTIVE_SCHED {
        Some(ref mut a) => a,
        None => return,
    };
    let cur = a.base_scheduler.current;
    if let Some(t) = a.base_scheduler.tasks[cur].as_mut() {
        t.state = if terminate {
            TaskState::Terminated
        } else {
            TaskState::Ready
        };
    }

    // Find the next runnable task (round-robin).
    let mut found = None;
    let mut idx = cur;
    for _ in 0..MAX_TASKS {
        idx = (idx + 1) % MAX_TASKS;
        if let Some(t) = a.base_scheduler.tasks[idx].as_ref() {
            if t.state == TaskState::Ready {
                found = Some(idx);
                break;
            }
        }
    }

    if let Some(idx) = found {
        a.base_scheduler.tasks[idx].as_mut().unwrap().state = TaskState::Running;
        a.base_scheduler.current = idx;
        CURRENT_TF = &mut a.base_scheduler.tasks[idx].as_mut().unwrap().tf as *mut TrapFrame;
    } else if !terminate {
        // Nothing else to run: keep running the current task.
        if let Some(t) = a.base_scheduler.tasks[cur].as_mut() {
            t.state = TaskState::Running;
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
