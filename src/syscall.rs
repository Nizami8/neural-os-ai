//! Syscall numbers and dispatcher. IPC path checks capabilities first.

use crate::capability::{CapError, Capability, RIGHT_GRANT, RIGHT_RECV, RIGHT_SEND};
use crate::ipc::Message;
use crate::kernel::Kernel;
use crate::thread::ThreadState;
use crate::trapframe::TrapFrame;

pub const SYS_YIELD: usize = 0;
pub const SYS_GETPID: usize = 1;
pub const SYS_GETTID: usize = 2;
pub const SYS_PRINT: usize = 3;
pub const SYS_EXIT: usize = 4;
pub const SYS_SEND: usize = 5;
pub const SYS_RECV: usize = 6;
pub const SYS_CREATE_ENDPOINT: usize = 7;
pub const SYS_GRANT: usize = 8;

pub const OK: usize = 0;
pub const ERR_NO_CAP: usize = usize::MAX;
pub const ERR_DENIED: usize = usize::MAX - 1;
pub const ERR_BAD_EP: usize = usize::MAX - 2;
pub const ERR_DEAD: usize = usize::MAX - 3;
pub const ERR_FULL: usize = usize::MAX - 4;
pub const ERR_BAD_SYS: usize = usize::MAX - 5;

impl From<CapError> for usize {
    fn from(e: CapError) -> Self {
        match e {
            CapError::Denied | CapError::Expired => ERR_DENIED,
            CapError::EmptySlot | CapError::BadIndex | CapError::WrongType | CapError::StaleGeneration => {
                ERR_NO_CAP
            }
            CapError::TableFull => ERR_FULL,
        }
    }
}

pub fn dispatch(kernel: &mut Kernel, tf: &mut TrapFrame) {
    tf.advance_pc();
    let nr = tf.syscall_number();
    let a0 = tf.arg0();
    let a1 = tf.arg1();
    let a2 = tf.arg2();
    let a3 = tf.arg3();

    let ret = match nr {
        SYS_YIELD => {
            kernel.want_resched = true;
            OK
        }
        SYS_GETPID => kernel.current_pid(),
        SYS_GETTID => kernel.cpu.current,
        SYS_PRINT => {
            kernel.sys_print(a0, a1);
            OK
        }
        SYS_EXIT => {
            kernel.sys_exit();
            kernel.want_resched = true;
            OK
        }
        SYS_SEND => kernel.sys_send(a0, a1, a2, a3),
        SYS_RECV => kernel.sys_recv(a0),
        SYS_CREATE_ENDPOINT => kernel.sys_create_endpoint(a0),
        SYS_GRANT => kernel.sys_grant(a0, a1, a2),
        _ => ERR_BAD_SYS,
    };

    if kernel.threads.slots[kernel.cpu.current].state == ThreadState::Running {
        tf.set_return(ret);
        if nr == SYS_RECV && ret == OK {
            let msg = kernel.threads.slots[kernel.cpu.current].ipc_msg;
            tf.set_ipc_return(OK, msg.words[0], msg.words[1], msg.words[2], msg.sender);
        }
    }
}

impl Kernel {
    pub fn sys_print(&mut self, ptr: usize, len: usize) {
        if len == 0 || len > 128 {
            return;
        }
        // Kernel threads pass a pointer in their own address space (identity map).
        let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, len) };
        if let Ok(s) = core::str::from_utf8(bytes) {
            crate::console::write_str(s);
        }
    }

    pub fn sys_exit(&mut self) {
        let tid = self.cpu.current;
        let pid = self.threads.slots[tid].pid;
        if let Some(ep) = self.threads.slots[tid].blocked_on {
            if let Ok(endpoint) = self.endpoints.get_mut(ep) {
                endpoint.cancel(tid);
            }
        }
        self.threads.slots[tid].state = ThreadState::Exited;
        self.threads.slots[tid].blocked_on = None;
        if let Some(proc) = self.processes.get_mut(pid) {
            if proc.primary_tid == tid {
                proc.alive = false;
            }
        }
        crate::console::write_str("[exit tid=");
        crate::console::write_usize(tid);
        crate::console::write_str("]\n");
    }

    pub fn sys_create_endpoint(&mut self, rights: usize) -> usize {
        let pid = self.current_pid();
        let ep = match self.endpoints.alloc() {
            Ok(id) => id,
            Err(_) => return ERR_FULL,
        };
        let gen = self.endpoints.slots[ep].generation;
        let cap_rights = if rights == 0 {
            RIGHT_SEND | RIGHT_RECV | RIGHT_GRANT
        } else {
            rights as u32
        };
        let cap = Capability::endpoint(ep as u16, gen, cap_rights, pid as u16);
        match self.processes.slots[pid].caps.insert(cap) {
            Ok(idx) => idx,
            Err(_) => {
                self.endpoints.slots[ep].revoke();
                ERR_FULL
            }
        }
    }

    pub fn sys_grant(&mut self, cap_idx: usize, dest_pid: usize, rights: usize) -> usize {
        let src = self.current_pid();
        let now = self.scheduler.tick;
        let src_cap = match self.processes.slots[src].caps.get(cap_idx) {
            Ok(c) => *c,
            Err(e) => return e.into(),
        };
        if !src_cap.has_right(RIGHT_GRANT) {
            return ERR_DENIED;
        }
        if src_cap.is_expired(now) {
            return ERR_DENIED;
        }
        let dest = match self.processes.get_mut(dest_pid) {
            Some(p) if p.alive => p,
            _ => return ERR_DEAD,
        };
        let granted = src_cap.restricted(rights as u32);
        match dest.caps.insert(granted) {
            Ok(idx) => idx,
            Err(_) => ERR_FULL,
        }
    }

    pub fn sys_send(&mut self, cap_idx: usize, w0: usize, w1: usize, w2: usize) -> usize {
        let tid = self.cpu.current;
        let pid = self.threads.slots[tid].pid;
        let now = self.scheduler.tick;
        let cap = match self.processes.slots[pid].caps.get(cap_idx) {
            Ok(c) => *c,
            Err(e) => return e.into(),
        };
        if cap.object_type != crate::capability::ObjectType::Endpoint {
            return ERR_NO_CAP;
        }
        if !cap.has_right(RIGHT_SEND) {
            self.ipc_denied += 1;
            return ERR_DENIED;
        }
        if cap.is_expired(now) {
            return ERR_DENIED;
        }
        let ep_id = cap.object_id as usize;
        let gen = match self.endpoints.get(ep_id) {
            Ok(ep) => ep.generation,
            Err(_) => return ERR_BAD_EP,
        };
        if cap.generation != gen {
            return ERR_NO_CAP;
        }

        let msg = Message::new(tid, w0, w1, w2);

        if let Some(recv_tid) = self.endpoints.slots[ep_id].pop_receiver() {
            self.complete_rendezvous(recv_tid, msg);
            self.ipc_sends += 1;
            return OK;
        }

        self.threads.slots[tid].ipc_msg = msg;
        self.threads.slots[tid].blocked_on = Some(ep_id);
        self.threads.slots[tid].state = ThreadState::BlockedSend;
        if self.endpoints.slots[ep_id].enqueue_sender(tid).is_err() {
            self.threads.slots[tid].state = ThreadState::Ready;
            self.threads.slots[tid].blocked_on = None;
            return ERR_FULL;
        }
        self.want_resched = true;
        self.ipc_blocks += 1;
        OK
    }

    pub fn sys_recv(&mut self, cap_idx: usize) -> usize {
        let tid = self.cpu.current;
        let pid = self.threads.slots[tid].pid;
        let now = self.scheduler.tick;
        let cap = match self.processes.slots[pid].caps.get(cap_idx) {
            Ok(c) => *c,
            Err(e) => return e.into(),
        };
        if cap.object_type != crate::capability::ObjectType::Endpoint {
            return ERR_NO_CAP;
        }
        if !cap.has_right(RIGHT_RECV) {
            self.ipc_denied += 1;
            return ERR_DENIED;
        }
        if cap.is_expired(now) {
            return ERR_DENIED;
        }
        let ep_id = cap.object_id as usize;
        let gen = match self.endpoints.get(ep_id) {
            Ok(ep) => ep.generation,
            Err(_) => return ERR_BAD_EP,
        };
        if cap.generation != gen {
            return ERR_NO_CAP;
        }

        if let Some(send_tid) = self.endpoints.slots[ep_id].pop_sender() {
            let msg = self.threads.slots[send_tid].ipc_msg;
            self.threads.slots[tid].ipc_msg = msg;
            self.wake(send_tid, OK);
            self.ipc_recvs += 1;
            return OK;
        }

        self.threads.slots[tid].blocked_on = Some(ep_id);
        self.threads.slots[tid].state = ThreadState::BlockedRecv;
        if self.endpoints.slots[ep_id].enqueue_receiver(tid).is_err() {
            self.threads.slots[tid].state = ThreadState::Ready;
            self.threads.slots[tid].blocked_on = None;
            return ERR_FULL;
        }
        self.want_resched = true;
        self.ipc_blocks += 1;
        OK
    }

    fn complete_rendezvous(&mut self, recv_tid: usize, msg: Message) {
        self.threads.slots[recv_tid].ipc_msg = msg;
        self.threads.slots[recv_tid]
            .trapframe
            .set_ipc_return(OK, msg.words[0], msg.words[1], msg.words[2], msg.sender);
        self.wake(recv_tid, OK);
        self.ipc_recvs += 1;
    }

    pub fn wake(&mut self, tid: usize, status: usize) {
        if tid == 0 || tid >= crate::config::MAX_THREADS {
            return;
        }
        let t = &mut self.threads.slots[tid];
        if t.state == ThreadState::BlockedSend || t.state == ThreadState::BlockedRecv {
            t.state = ThreadState::Ready;
            t.blocked_on = None;
            t.trapframe.set_return(status);
            if status == OK && t.ipc_msg.sender != 0 {
                let msg = t.ipc_msg;
                t.trapframe
                    .set_ipc_return(OK, msg.words[0], msg.words[1], msg.words[2], msg.sender);
            }
        }
    }
}
