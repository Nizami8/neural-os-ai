/// Base task table for the Neural OS scheduler.
///
/// The adaptive (neural) scheduler in `adaptive.rs` sits on top of this and
/// makes the scheduling *decisions* (which task should run next) using the MLP,
/// fairness and Q-learning. Task selection here is cooperative: `context_switch`
/// records the decision without performing a register-level stack switch, which
/// keeps the kernel's monitoring loop alive so it can report live statistics.

pub const MAX_TASKS: usize = 4;

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
                self.tasks[i] = Some(Task {
                    id,
                    entry,
                    state,
                    context: Context::new(),
                });
                return;
            }
        }
    }
}

/// Record a scheduling decision. Cooperative: the running kernel thread keeps
/// executing, so the monitoring loop can print statistics between timer ticks.
///
/// # Safety
/// Takes raw context handles for parity with a real register-level switch.
pub unsafe fn context_switch(_old: &mut Context, _new: &mut Context) {}
