//! Synchronous message-passing IPC (Stage 4).
//!
//! A tiny rendezvous primitive in the style of L4/seL4 synchronous IPC: a task
//! `send`s a one-word message on an endpoint and a task `recv`s from it. Whoever
//! arrives first blocks (task state `Blocked`) until its partner appears, at
//! which point the message is transferred directly and both continue. The
//! scheduler never selects a `Blocked` task, so blocking parks it cleanly.
//!
//! This minimal version keeps a single waiting sender and a single waiting
//! receiver per endpoint, which is enough for one producer/consumer pair.

pub const NUM_ENDPOINTS: usize = 4;

// Capability rights a task may hold on an endpoint (seL4-style access control).
pub const CAP_SEND: u8 = 1 << 0;
pub const CAP_RECV: u8 = 1 << 1;

// Return codes for send (recv returns the message, or RECV_DENIED on error).
pub const OK: usize = 0;
pub const EPERM: usize = 1;
pub const RECV_DENIED: usize = usize::MAX;

#[derive(Clone, Copy)]
pub struct Endpoint {
    /// A blocked sender: (task index, message).
    pub sender: Option<(usize, usize)>,
    /// A blocked receiver: task index.
    pub receiver: Option<usize>,
}

impl Endpoint {
    pub const fn new() -> Self {
        Endpoint {
            sender: None,
            receiver: None,
        }
    }
}

pub static ENDPOINTS: crate::kcell::KernelCell<[Endpoint; NUM_ENDPOINTS]> =
    crate::kcell::KernelCell::new([Endpoint::new(); NUM_ENDPOINTS]);
