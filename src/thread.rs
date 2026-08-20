//! Thread Control Block. Scheduler, IPC, and trap all talk to this type.

use crate::config::{DEFAULT_QUANTUM, KERNEL_STACK_SIZE, MAX_THREADS};
use crate::context::Context;
use crate::ipc::Message;
use crate::neural::TaskClass;
use crate::trapframe::TrapFrame;

pub type ThreadId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadState {
    Unused,
    Ready,
    Running,
    BlockedSend,
    BlockedRecv,
    Sleeping,
    Exited,
}

/// Compatibility alias used by the AI observer.
pub type TaskState = ThreadState;

pub struct Thread {
    pub id: ThreadId,
    pub pid: usize,
    pub state: ThreadState,
    pub priority: u8,
    pub quantum: usize,
    pub ticks_used: usize,
    pub trapframe: TrapFrame,
    pub context: Context,
    pub kernel_stack: [u8; KERNEL_STACK_SIZE],
    pub ipc_msg: Message,
    pub blocked_on: Option<usize>,
    pub class: TaskClass,
    pub cpu_time: u32,
    pub wait_time: u32,
}

impl Thread {
    pub const fn unused() -> Self {
        Self {
            id: 0,
            pid: 0,
            state: ThreadState::Unused,
            priority: 0,
            quantum: DEFAULT_QUANTUM,
            ticks_used: 0,
            trapframe: TrapFrame::zero(),
            context: Context::zero(),
            kernel_stack: [0; KERNEL_STACK_SIZE],
            ipc_msg: Message::empty(),
            blocked_on: None,
            class: TaskClass::Batch,
            cpu_time: 0,
            wait_time: 0,
        }
    }

    pub fn stack_top(&self) -> usize {
        let base = self.kernel_stack.as_ptr() as usize;
        (base + KERNEL_STACK_SIZE) & !0xF
    }

    pub fn activate(
        &mut self,
        id: ThreadId,
        pid: usize,
        entry: usize,
        class: TaskClass,
        priority: u8,
    ) {
        self.id = id;
        self.pid = pid;
        self.state = ThreadState::Ready;
        self.priority = priority;
        self.quantum = DEFAULT_QUANTUM;
        self.ticks_used = 0;
        self.ipc_msg = Message::empty();
        self.blocked_on = None;
        self.class = class;
        self.cpu_time = 0;
        self.wait_time = 0;
        let sp = self.stack_top();
        self.trapframe.init_kernel(entry, sp);
        self.context.init(entry, sp);
    }

    pub fn is_runnable(&self) -> bool {
        self.state == ThreadState::Ready || self.state == ThreadState::Running
    }
}

pub struct ThreadTable {
    pub slots: [Thread; MAX_THREADS],
}

impl ThreadTable {
    pub const fn new() -> Self {
        Self {
            slots: [const { Thread::unused() }; MAX_THREADS],
        }
    }

    pub fn alloc(&mut self) -> Option<ThreadId> {
        for i in 1..MAX_THREADS {
            if self.slots[i].state == ThreadState::Unused {
                return Some(i);
            }
        }
        None
    }

    pub fn get(&self, id: ThreadId) -> Option<&Thread> {
        self.slots.get(id).filter(|t| t.state != ThreadState::Unused)
    }

    pub fn get_mut(&mut self, id: ThreadId) -> Option<&mut Thread> {
        self.slots
            .get_mut(id)
            .filter(|t| t.state != ThreadState::Unused)
    }
}
