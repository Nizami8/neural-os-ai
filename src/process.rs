//! Process object: owns a capability table and a primary thread.

use crate::capability::CapabilityTable;
use crate::config::MAX_PROCESSES;
use crate::thread::ThreadId;

pub type ProcessId = usize;

pub struct Process {
    pub pid: ProcessId,
    pub used: bool,
    pub alive: bool,
    pub caps: CapabilityTable,
    pub primary_tid: ThreadId,
}

impl Process {
    pub const fn empty() -> Self {
        Self {
            pid: 0,
            used: false,
            alive: false,
            caps: CapabilityTable::new(),
            primary_tid: 0,
        }
    }
}

pub struct ProcessTable {
    pub slots: [Process; MAX_PROCESSES],
}

impl ProcessTable {
    pub const fn new() -> Self {
        Self {
            slots: [const { Process::empty() }; MAX_PROCESSES],
        }
    }

    pub fn alloc(&mut self) -> Option<ProcessId> {
        for i in 1..MAX_PROCESSES {
            if !self.slots[i].used {
                self.slots[i] = Process::empty();
                self.slots[i].used = true;
                self.slots[i].alive = true;
                self.slots[i].pid = i;
                return Some(i);
            }
        }
        None
    }

    pub fn get(&self, pid: ProcessId) -> Option<&Process> {
        self.slots.get(pid).filter(|p| p.used)
    }

    pub fn get_mut(&mut self, pid: ProcessId) -> Option<&mut Process> {
        self.slots.get_mut(pid).filter(|p| p.used)
    }
}
