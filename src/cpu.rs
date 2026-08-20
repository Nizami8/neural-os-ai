//! Per-hart CPU local state.

use crate::thread::ThreadId;
use crate::trapframe::TrapFrame;

pub struct Cpu {
    pub hartid: usize,
    pub current: ThreadId,
    pub interrupt_depth: usize,
    pub scheduler_enabled: bool,
}

impl Cpu {
    pub const fn new() -> Self {
        Self {
            hartid: 0,
            current: 0,
            interrupt_depth: 0,
            scheduler_enabled: false,
        }
    }

    pub fn current_tf<'a>(&self, threads: &'a mut [crate::thread::Thread]) -> Option<&'a mut TrapFrame> {
        threads.get_mut(self.current).map(|t| &mut t.trapframe)
    }
}
