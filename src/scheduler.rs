/// Base task table for the Neural OS scheduler.
///
/// The adaptive (neural) scheduler in `adaptive.rs` sits on top of this and
/// makes the scheduling *decisions* (which task should run next) using the MLP,
/// fairness and Q-learning. The actual switch is preemptive: every task owns a
/// full `TrapFrame` and its own stack, and the timer trap vector saves/restores
/// those frames (see `trap.s` / `src/trap.rs`).

use crate::trapframe::{TrapFrame, MSTATUS_INIT};
use crate::ipc::NUM_ENDPOINTS;

pub const MAX_TASKS: usize = 7;

/// Per-task stack size (16 KiB). Task code is tiny; the neural scheduler runs on
/// the separate kernel stack inside the trap handler.
pub const TASK_STACK_SIZE: usize = 16 * 1024;

#[repr(C, align(16))]
struct TaskStack([u8; TASK_STACK_SIZE]);

static mut TASK_STACKS: [TaskStack; MAX_TASKS] =
    [const { TaskStack([0; TASK_STACK_SIZE]) }; MAX_TASKS];

/// Top-of-stack address (16-byte aligned) for task slot `i`.
fn stack_top(i: usize) -> usize {
    let base = unsafe { core::ptr::addr_of!(TASK_STACKS[i]) as usize };
    (base + TASK_STACK_SIZE) & !0xF
}

#[derive(Clone, Copy, PartialEq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

/// Saved execution context for a task. Kept minimal because scheduling is
/// cooperative; it exists so the adaptive scheduler has a stable handle to pass
/// to `context_switch`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Context {
    pub sp: usize,
    pub ra: usize,
}

impl Context {
    pub const fn new() -> Self {
        Context { sp: 0, ra: 0 }
    }
}

#[derive(Clone, Copy)]
pub struct Task {
    pub id: usize,
    pub entry: fn(),
    pub state: TaskState,
    pub context: Context,
    /// Full saved register state; the timer trap vector switches through this.
    pub tf: TrapFrame,
    /// Per-endpoint IPC capability rights (CAP_SEND | CAP_RECV).
    pub caps: [u8; NUM_ENDPOINTS],
}

pub struct Scheduler {
    pub tasks: [Option<Task>; MAX_TASKS],
    pub current: usize,
    pub tick: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Scheduler {
            tasks: [None; MAX_TASKS],
            current: 0,
            tick: 0,
        }
    }

    pub fn add_task(&mut self, id: usize, entry: fn()) {
        for i in 0..MAX_TASKS {
            if self.tasks[i].is_none() {
                let state = if i == 0 {
                    TaskState::Running
                } else {
                    TaskState::Ready
                };

                // Prime the task's TrapFrame so the first mret enters `entry`
                // on the task's own stack with interrupts + FPU enabled.
                let mut tf = TrapFrame::zero();
                tf.regs[2] = stack_top(i); // sp
                tf.mepc = entry as usize; // resume PC
                tf.mstatus = MSTATUS_INIT;

                self.tasks[i] = Some(Task {
                    id,
                    entry,
                    state,
                    context: Context::new(),
                    tf,
                    caps: [0; NUM_ENDPOINTS],
                });
                return;
            }
        }
    }

    /// Grant a task (by id) capability `rights` on `endpoint`.
    pub fn grant(&mut self, id: usize, endpoint: usize, rights: u8) {
        if endpoint >= NUM_ENDPOINTS {
            return;
        }
        for i in 0..MAX_TASKS {
            if let Some(t) = self.tasks[i].as_mut() {
                if t.id == id {
                    t.caps[endpoint] |= rights;
                }
            }
        }
    }
}

/// Kept for source compatibility with the adaptive scheduler's decision path.
/// The real register-level switch now happens in the timer trap vector via each
/// task's `TrapFrame`, so this is a no-op.
///
/// # Safety
/// Takes raw context handles for parity with a register-level switch.
pub unsafe fn context_switch(_old: &mut Context, _new: &mut Context) {}
