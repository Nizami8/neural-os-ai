//! L4-style synchronous IPC: capability → endpoint → rendezvous.

#![allow(dead_code)]

use crate::config::{MAX_ENDPOINTS, WAIT_QUEUE};

pub type EndpointId = usize;
pub type ThreadId = usize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Message {
    pub words: [usize; 3],
    pub sender: ThreadId,
}

impl Message {
    pub const fn empty() -> Self {
        Self {
            words: [0; 3],
            sender: 0,
        }
    }

    pub fn new(sender: ThreadId, w0: usize, w1: usize, w2: usize) -> Self {
        Self {
            words: [w0, w1, w2],
            sender,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpcError {
    BadEndpoint,
    QueueFull,
    NoPartner,
}

#[derive(Clone, Copy)]
struct WaitQueue {
    items: [ThreadId; WAIT_QUEUE],
    head: usize,
    len: usize,
}

impl WaitQueue {
    const fn new() -> Self {
        Self {
            items: [0; WAIT_QUEUE],
            head: 0,
            len: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn push(&mut self, tid: ThreadId) -> Result<(), IpcError> {
        if self.len >= WAIT_QUEUE {
            return Err(IpcError::QueueFull);
        }
        let idx = (self.head + self.len) % WAIT_QUEUE;
        self.items[idx] = tid;
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<ThreadId> {
        if self.len == 0 {
            return None;
        }
        let tid = self.items[self.head];
        self.head = (self.head + 1) % WAIT_QUEUE;
        self.len -= 1;
        Some(tid)
    }

    fn contains(&self, tid: ThreadId) -> bool {
        for i in 0..self.len {
            if self.items[(self.head + i) % WAIT_QUEUE] == tid {
                return true;
            }
        }
        false
    }

    fn remove(&mut self, tid: ThreadId) -> bool {
        if self.len == 0 {
            return false;
        }
        let mut tmp = [0; WAIT_QUEUE];
        let mut n = 0;
        let mut found = false;
        for i in 0..self.len {
            let item = self.items[(self.head + i) % WAIT_QUEUE];
            if item == tid && !found {
                found = true;
                continue;
            }
            tmp[n] = item;
            n += 1;
        }
        self.items = tmp;
        self.head = 0;
        self.len = n;
        found
    }
}

#[derive(Clone, Copy)]
pub struct Endpoint {
    pub used: bool,
    pub generation: u32,
    senders: WaitQueue,
    receivers: WaitQueue,
}

impl Endpoint {
    pub const fn empty() -> Self {
        Self {
            used: false,
            generation: 0,
            senders: WaitQueue::new(),
            receivers: WaitQueue::new(),
        }
    }

    pub fn activate(&mut self) {
        self.used = true;
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.generation = 1;
        }
        self.senders = WaitQueue::new();
        self.receivers = WaitQueue::new();
    }

    pub fn revoke(&mut self) {
        self.used = false;
        self.generation = self.generation.wrapping_add(1);
        self.senders = WaitQueue::new();
        self.receivers = WaitQueue::new();
    }

    pub fn pop_receiver(&mut self) -> Option<ThreadId> {
        self.receivers.pop()
    }

    pub fn pop_sender(&mut self) -> Option<ThreadId> {
        self.senders.pop()
    }

    pub fn enqueue_sender(&mut self, tid: ThreadId) -> Result<(), IpcError> {
        if !self.used {
            return Err(IpcError::BadEndpoint);
        }
        self.senders.push(tid)
    }

    pub fn enqueue_receiver(&mut self, tid: ThreadId) -> Result<(), IpcError> {
        if !self.used {
            return Err(IpcError::BadEndpoint);
        }
        self.receivers.push(tid)
    }

    pub fn has_waiting_receiver(&self) -> bool {
        !self.receivers.is_empty()
    }

    pub fn has_waiting_sender(&self) -> bool {
        !self.senders.is_empty()
    }

    pub fn cancel(&mut self, tid: ThreadId) {
        let _ = self.senders.remove(tid);
        let _ = self.receivers.remove(tid);
    }

    pub fn waiting_sender_is(&self, tid: ThreadId) -> bool {
        self.senders.contains(tid)
    }

    pub fn waiting_receiver_is(&self, tid: ThreadId) -> bool {
        self.receivers.contains(tid)
    }
}

pub struct EndpointTable {
    pub slots: [Endpoint; MAX_ENDPOINTS],
}

impl EndpointTable {
    pub const fn new() -> Self {
        Self {
            slots: [Endpoint::empty(); MAX_ENDPOINTS],
        }
    }

    pub fn alloc(&mut self) -> Result<EndpointId, IpcError> {
        for i in 1..MAX_ENDPOINTS {
            if !self.slots[i].used {
                self.slots[i].activate();
                return Ok(i);
            }
        }
        Err(IpcError::BadEndpoint)
    }

    pub fn get(&self, id: EndpointId) -> Result<&Endpoint, IpcError> {
        if id == 0 || id >= MAX_ENDPOINTS || !self.slots[id].used {
            Err(IpcError::BadEndpoint)
        } else {
            Ok(&self.slots[id])
        }
    }

    pub fn get_mut(&mut self, id: EndpointId) -> Result<&mut Endpoint, IpcError> {
        if id == 0 || id >= MAX_ENDPOINTS || !self.slots[id].used {
            Err(IpcError::BadEndpoint)
        } else {
            Ok(&mut self.slots[id])
        }
    }

    pub fn generation(&self, id: EndpointId) -> Result<u32, IpcError> {
        Ok(self.get(id)?.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendezvous_prefers_waiting_receiver() {
        let mut ep = Endpoint::empty();
        ep.activate();
        ep.enqueue_receiver(2).unwrap();
        assert_eq!(ep.pop_receiver(), Some(2));
        assert!(!ep.has_waiting_receiver());
    }

    #[test]
    fn fifo_waiters() {
        let mut ep = Endpoint::empty();
        ep.activate();
        ep.enqueue_sender(3).unwrap();
        ep.enqueue_sender(4).unwrap();
        assert_eq!(ep.pop_sender(), Some(3));
        assert_eq!(ep.pop_sender(), Some(4));
    }
}
