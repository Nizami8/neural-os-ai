use core::arch::asm;
use core::sync::atomic::Ordering;
use crate::kcell::KernelCell;
use crate::trapframe::TrapFrame;
use crate::scheduler::{TaskState, MAX_TASKS};
use crate::syscall;
use crate::ipc;
use crate::ADAPTIVE_SCHED;
use crate::USE_AI;
use crate::STORAGE;

// QEMU `virt` CLINT (Core Local Interruptor) registers for hart 0.
const CLINT_MTIME: *mut u64 = 0x0200_bff8 as *mut u64;
const CLINT_MTIMECMP: *mut u64 = 0x0200_4000 as *mut u64;

// mtime runs at 10 MHz on QEMU virt; time slice between preemptions.
const TIMER_INTERVAL: u64 = 100_000; // ~10 ms quantum

// Print an automatic statistics block every N timer ticks. Kept infrequent so
// it does not clutter the interactive shell (use the `stats` command on demand).
const STATS_EVERY: usize = 2000;

/// Persist learned weights every N statistics dumps.
const PERSIST_EVERY_STATS: usize = 5;

/// Pointer to the TrapFrame of the currently running task. Read and written by
/// the assembly trap vector (trap.s) and updated here when the scheduler picks
/// a new task. `KernelCell` is `#[repr(transparent)]` so the symbol layout
/// matches a plain pointer for the assembly `la`/`ld` sequence.
#[no_mangle]
pub static CURRENT_TF: KernelCell<*mut TrapFrame> = KernelCell::new(core::ptr::null_mut());

static TICKS: KernelCell<usize> = KernelCell::new(0);
static STAT_DUMPS: KernelCell<usize> = KernelCell::new(0);

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

unsafe fn persist_weights_if_due() {
    *STAT_DUMPS.as_mut() += 1;
    if *STAT_DUMPS.as_ref() % PERSIST_EVERY_STATS != 0 {
        return;
    }
    let Some(adaptive) = ADAPTIVE_SCHED.as_ref().as_ref() else {
        return;
    };
    let Some(storage) = STORAGE.as_mut().as_mut() else {
        return;
    };
    let (hidden, output) = adaptive.neural.export_weights();
    storage.save_weights(&hidden, &output);
}

unsafe fn on_timer_tick() {
    if USE_AI.load(Ordering::Relaxed) == 1 {
        if let Some(ref mut adaptive) = *ADAPTIVE_SCHED.as_mut() {
            // Neural / fairness / Q-learning picks the next task.
            adaptive.adaptive_schedule();

            // Point the trap vector at the newly selected task's frame.
            let cur = adaptive.base_scheduler.current;
            if let Some(ref mut task) = adaptive.base_scheduler.tasks[cur] {
                *CURRENT_TF.as_mut() = &mut task.tf as *mut TrapFrame;
            }

            *TICKS.as_mut() += 1;
            if *TICKS.as_ref() % STATS_EVERY == 0 {
                adaptive.collect_statistics();
                crate::print_stats(adaptive);
                persist_weights_if_due();
            }
        }
    }
}

/// Decode and service an `ecall` from the running task. Uses the saved
/// TrapFrame (CURRENT_TF) for arguments/return value and advances mepc past the
/// ecall so the task resumes at the following instruction.
unsafe fn handle_syscall() {
    let tf = *CURRENT_TF.as_ref();
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
        syscall::SYS_YIELD => reschedule(TaskState::Ready),
        syscall::SYS_EXIT => reschedule(TaskState::Terminated),
        syscall::SYS_SEND => ipc_send(arg0, arg1),
        syscall::SYS_RECV => ipc_recv(arg0),
        syscall::SYS_PS => {
            if let Some(a) = ADAPTIVE_SCHED.as_ref().as_ref() {
                crate::print_ps(a);
            }
        }
        syscall::SYS_STATS => {
            if let Some(a) = ADAPTIVE_SCHED.as_mut().as_mut() {
                a.collect_statistics();
                crate::print_stats(a);
            }
        }
        syscall::SYS_NN => {
            if let Some(a) = ADAPTIVE_SCHED.as_ref().as_ref() {
                crate::print_nn(a);
            }
        }
        _ => {}
    }
}

/// Move the current task to `new_state` and switch to the next Ready task
/// (round-robin). Used by yield (Ready), exit (Terminated) and blocking IPC
/// (Blocked). Relies on at least one task always being runnable.
unsafe fn reschedule(new_state: TaskState) {
    let a = match ADAPTIVE_SCHED.as_mut().as_mut() {
        Some(a) => a,
        None => return,
    };
    let cur = a.base_scheduler.current;
    if let Some(t) = a.base_scheduler.tasks[cur].as_mut() {
        t.state = new_state;
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
        *CURRENT_TF.as_mut() =
            &mut a.base_scheduler.tasks[idx].as_mut().unwrap().tf as *mut TrapFrame;
    } else if new_state == TaskState::Ready {
        // Yield with nothing else to run: keep running the current task.
        if let Some(t) = a.base_scheduler.tasks[cur].as_mut() {
            t.state = TaskState::Running;
        }
    }
}

/// Does the current task hold `right` on endpoint `epid`?
unsafe fn current_has_cap(epid: usize, right: u8) -> bool {
    ADAPTIVE_SCHED.as_ref().as_ref().map_or(false, |a| {
        let cur = a.base_scheduler.current;
        a.base_scheduler.tasks[cur]
            .as_ref()
            .map_or(false, |t| t.caps[epid] & right != 0)
    })
}

/// SYS_SEND: rendezvous send of a one-word message on `epid`.
unsafe fn ipc_send(epid: usize, msg: usize) {
    if epid >= ipc::NUM_ENDPOINTS || !current_has_cap(epid, ipc::CAP_SEND) {
        (*(*CURRENT_TF.as_ref())).regs[10] = ipc::EPERM;
        return;
    }
    let endpoints = ipc::ENDPOINTS.as_mut();
    if let Some(recv_idx) = endpoints[epid].receiver.take() {
        // A receiver was waiting: deliver directly and wake it.
        if let Some(a) = ADAPTIVE_SCHED.as_mut().as_mut() {
            if let Some(rt) = a.base_scheduler.tasks[recv_idx].as_mut() {
                rt.tf.regs[10] = msg; // recv() returns the message
                rt.state = TaskState::Ready;
            }
            let cur = a.base_scheduler.current;
            if let Some(st) = a.base_scheduler.tasks[cur].as_mut() {
                st.tf.regs[10] = 0; // send() returns success; sender continues
            }
        }
    } else {
        // No receiver yet: park this sender until one arrives.
        let cur = ADAPTIVE_SCHED
            .as_ref()
            .as_ref()
            .map(|a| a.base_scheduler.current)
            .unwrap_or(0);
        endpoints[epid].sender = Some((cur, msg));
        reschedule(TaskState::Blocked);
    }
}

/// SYS_RECV: rendezvous receive of a one-word message from `epid`.
unsafe fn ipc_recv(epid: usize) {
    if epid >= ipc::NUM_ENDPOINTS || !current_has_cap(epid, ipc::CAP_RECV) {
        (*(*CURRENT_TF.as_ref())).regs[10] = ipc::RECV_DENIED;
        return;
    }
    let endpoints = ipc::ENDPOINTS.as_mut();
    if let Some((send_idx, msg)) = endpoints[epid].sender.take() {
        // A sender was waiting: take its message and wake it.
        if let Some(a) = ADAPTIVE_SCHED.as_mut().as_mut() {
            if let Some(stk) = a.base_scheduler.tasks[send_idx].as_mut() {
                stk.tf.regs[10] = 0; // send() returns success
                stk.state = TaskState::Ready;
            }
            let cur = a.base_scheduler.current;
            if let Some(rt) = a.base_scheduler.tasks[cur].as_mut() {
                rt.tf.regs[10] = msg; // recv() returns the message; receiver continues
            }
        }
    } else {
        // No sender yet: park this receiver until one arrives.
        let cur = ADAPTIVE_SCHED
            .as_ref()
            .as_ref()
            .map(|a| a.base_scheduler.current)
            .unwrap_or(0);
        endpoints[epid].receiver = Some(cur);
        reschedule(TaskState::Blocked);
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
