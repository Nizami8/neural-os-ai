//! Round-robin scheduler over TCBs. AI may adjust `priority`, not pick the CPU.

use crate::config::MAX_THREADS;
use crate::cpu::Cpu;
use crate::thread::{Thread, ThreadId, ThreadState, ThreadTable};

pub struct Scheduler {
    pub current: ThreadId,
    pub tick: u64,
    rr_cursor: ThreadId,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            current: 0,
            tick: 0,
            rr_cursor: 1,
        }
    }

    pub fn pick_next(&mut self, threads: &ThreadTable) -> ThreadId {
        let start = self.rr_cursor;
        let mut i = start;
        loop {
            if threads.slots[i].state == ThreadState::Ready {
                self.rr_cursor = if i + 1 < MAX_THREADS { i + 1 } else { 1 };
                return i;
            }
            i += 1;
            if i >= MAX_THREADS {
                i = 1;
            }
            if i == start {
                break;
            }
        }
        // Idle is slot 1 by convention if nothing else is ready.
        for (idx, t) in threads.slots.iter().enumerate().skip(1) {
            if t.state == ThreadState::Ready || t.state == ThreadState::Running {
                return idx;
            }
        }
        self.current
    }

    /// Highest neural/manual priority among Ready threads; ties broken by RR.
    pub fn pick_next_priority(&mut self, threads: &ThreadTable) -> ThreadId {
        let mut best_id = 0;
        let mut best_prio = -1i32;
        for i in 1..MAX_THREADS {
            let t = &threads.slots[i];
            if t.state == ThreadState::Ready || (t.state == ThreadState::Running && i == self.current)
            {
                let p = t.priority as i32;
                if p > best_prio {
                    best_prio = p;
                    best_id = i;
                }
            }
        }
        if best_id == 0 {
            self.pick_next(threads)
        } else {
            best_id
        }
    }

    pub fn preempt_needed(&self, threads: &ThreadTable) -> bool {
        let cur = &threads.slots[self.current];
        cur.ticks_used >= cur.quantum
    }
}

pub fn switch_to(cpu: &mut Cpu, threads: &mut [Thread], next: ThreadId) {
    let prev = cpu.current;
    if prev != 0 && prev < threads.len() && threads[prev].state == ThreadState::Running {
        threads[prev].state = ThreadState::Ready;
        threads[prev].ticks_used = 0;
    }
    cpu.current = next;
    if next != 0 && next < threads.len() {
        threads[next].state = ThreadState::Running;
        threads[next].ticks_used = 0;
    }
}

/// Compatibility 4-slot table used by the AI observer (host tests + metrics).
#[derive(Clone, Copy)]
pub struct Task {
    pub id: usize,
    pub state: ThreadState,
    pub context: crate::context::Context,
}

pub struct CompatScheduler {
    pub tasks: [Option<Task>; 4],
    pub current: usize,
    pub tick: usize,
}

impl CompatScheduler {
    pub const fn new() -> Self {
        Self {
            tasks: [None, None, None, None],
            current: 0,
            tick: 0,
        }
    }

    pub fn add_task(&mut self, id: usize, _entry: fn()) {
        if id == 0 || id > 4 {
            return;
        }
        self.tasks[id - 1] = Some(Task {
            id,
            state: ThreadState::Ready,
            context: crate::context::Context::zero(),
        });
        if self.current == 0 {
            self.current = id - 1;
            if let Some(t) = self.tasks[id - 1].as_mut() {
                t.state = ThreadState::Running;
            }
        }
    }
}

pub unsafe fn context_switch(old: *mut crate::context::Context, new: *mut crate::context::Context) {
    crate::context::context_switch(old, new as *const crate::context::Context);
}
