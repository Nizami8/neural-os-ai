//! Kernel object graph: CPU + processes + threads + endpoints + scheduler.

use crate::capability::{Capability, RIGHT_GRANT, RIGHT_RECV, RIGHT_SEND};
use crate::config::MAX_THREADS;
use crate::cpu::Cpu;
use crate::ipc::EndpointTable;
use crate::neural::TaskClass;
use crate::process::{ProcessId, ProcessTable};
use crate::scheduler::{switch_to, Scheduler};
use crate::thread::{ThreadId, ThreadState, ThreadTable};
use crate::trapframe::TrapFrame;

pub static mut KERNEL: Kernel = Kernel::new();

pub struct Kernel {
    pub cpu: Cpu,
    pub threads: ThreadTable,
    pub processes: ProcessTable,
    pub endpoints: EndpointTable,
    pub scheduler: Scheduler,
    pub want_resched: bool,
    pub ipc_sends: u64,
    pub ipc_recvs: u64,
    pub ipc_blocks: u64,
    pub ipc_denied: u64,
    pub context_switches: u64,
}

impl Kernel {
    pub const fn new() -> Self {
        Self {
            cpu: Cpu::new(),
            threads: ThreadTable::new(),
            processes: ProcessTable::new(),
            endpoints: EndpointTable::new(),
            scheduler: Scheduler::new(),
            want_resched: false,
            ipc_sends: 0,
            ipc_recvs: 0,
            ipc_blocks: 0,
            ipc_denied: 0,
            context_switches: 0,
        }
    }

    pub fn current_pid(&self) -> ProcessId {
        self.threads.slots[self.cpu.current].pid
    }

    pub fn spawn(
        &mut self,
        entry: usize,
        class: TaskClass,
        priority: u8,
    ) -> Option<(ProcessId, ThreadId)> {
        let pid = self.processes.alloc()?;
        let tid = self.threads.alloc()?;
        self.threads.slots[tid].activate(tid, pid, entry, class, priority);
        self.processes.slots[pid].primary_tid = tid;
        Some((pid, tid))
    }

    pub fn install_endpoint_caps(
        &mut self,
        ep: usize,
        server_pid: ProcessId,
        client_pid: ProcessId,
    ) -> Result<(usize, usize), ()> {
        let gen = self.endpoints.get(ep).map_err(|_| ())?.generation;
        let server_cap = Capability::endpoint(
            ep as u16,
            gen,
            RIGHT_SEND | RIGHT_RECV | RIGHT_GRANT,
            server_pid as u16,
        );
        let client_cap = Capability::endpoint(ep as u16, gen, RIGHT_SEND, client_pid as u16);
        let s = self.processes.slots[server_pid]
            .caps
            .insert(server_cap)
            .map_err(|_| ())?;
        let c = self.processes.slots[client_pid]
            .caps
            .insert(client_cap)
            .map_err(|_| ())?;
        Ok((s, c))
    }

    pub fn reschedule(&mut self) -> ThreadId {
        let next = self.scheduler.pick_next(&self.threads);
        if next != self.cpu.current {
            switch_to(&mut self.cpu, &mut self.threads.slots, next);
            self.scheduler.current = next;
            self.context_switches += 1;
        } else if self.threads.slots[next].state == ThreadState::Ready {
            self.threads.slots[next].state = ThreadState::Running;
            self.cpu.current = next;
            self.scheduler.current = next;
        }
        self.want_resched = false;
        self.cpu.current
    }

    pub fn on_timer(&mut self) {
        self.scheduler.tick += 1;
        let cur = self.cpu.current;
        if cur != 0 {
            self.threads.slots[cur].ticks_used += 1;
            self.threads.slots[cur].cpu_time = self.threads.slots[cur].cpu_time.saturating_add(1);
        }
        for i in 1..MAX_THREADS {
            if self.threads.slots[i].state == ThreadState::Ready {
                self.threads.slots[i].wait_time =
                    self.threads.slots[i].wait_time.saturating_add(1);
            }
        }
        if self.scheduler.preempt_needed(&self.threads) {
            self.want_resched = true;
        }
    }

    pub fn current_trapframe_ptr(&mut self) -> *mut TrapFrame {
        &mut self.threads.slots[self.cpu.current].trapframe as *mut TrapFrame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::RIGHT_RECV;
    use crate::syscall::{ERR_DENIED, OK, SYS_RECV, SYS_SEND, SYS_YIELD};

    fn dummy_entry() {}

    fn setup_pair() -> (Kernel, usize, usize, usize) {
        let mut k = Kernel::new();
        let entry = dummy_entry as usize;
        let (server_pid, server_tid) = k.spawn(entry, TaskClass::Interactive, 8).unwrap();
        let (client_pid, client_tid) = k.spawn(entry, TaskClass::Batch, 4).unwrap();
        let ep = k.endpoints.alloc().unwrap();
        let (sc, cc) = k.install_endpoint_caps(ep, server_pid, client_pid).unwrap();
        k.cpu.current = server_tid;
        k.scheduler.current = server_tid;
        k.threads.slots[server_tid].state = ThreadState::Running;
        let _ = client_tid;
        let _ = sc;
        (k, server_tid, client_tid, cc)
    }

    #[test]
    fn blocking_recv_then_send_rendezvous() {
        let (mut k, server, client, client_cap) = setup_pair();
        k.cpu.current = server;
        let status = k.sys_recv(1);
        assert_eq!(status, OK);
        assert_eq!(k.threads.slots[server].state, ThreadState::BlockedRecv);

        k.cpu.current = client;
        k.threads.slots[client].state = ThreadState::Running;
        let status = k.sys_send(client_cap, 42, 7, 1);
        assert_eq!(status, OK);
        assert_eq!(k.threads.slots[server].state, ThreadState::Ready);
        assert_eq!(k.threads.slots[server].ipc_msg.words[0], 42);
        assert_eq!(k.threads.slots[server].ipc_msg.words[1], 7);
        assert_eq!(k.threads.slots[server].ipc_msg.sender, client);
        assert_eq!(k.threads.slots[client].state, ThreadState::Running);
    }

    #[test]
    fn send_denied_without_send_right() {
        let (mut k, server, _client, _cc) = setup_pair();
        k.processes.slots[k.threads.slots[server].pid]
            .caps
            .insert_at(
                2,
                Capability::endpoint(1, k.endpoints.slots[1].generation, RIGHT_RECV, 1),
            )
            .unwrap();
        k.cpu.current = server;
        let status = k.sys_send(2, 1, 0, 0);
        assert_eq!(status, ERR_DENIED);
        assert_eq!(k.ipc_denied, 1);
    }

    #[test]
    fn yield_requests_reschedule() {
        let mut k = Kernel::new();
        let (_, tid) = k.spawn(dummy_entry as usize, TaskClass::Batch, 1).unwrap();
        k.cpu.current = tid;
        k.threads.slots[tid].state = ThreadState::Running;
        let mut tf = TrapFrame::zero();
        tf.a7 = SYS_YIELD;
        tf.mepc = 100;
        crate::syscall::dispatch(&mut k, &mut tf);
        assert!(k.want_resched);
        assert_eq!(tf.mepc, 104);
        assert_eq!(tf.a0, OK);
    }

    #[test]
    fn send_without_recv_blocks_sender() {
        let (mut k, _server, client, client_cap) = setup_pair();
        k.cpu.current = client;
        k.threads.slots[client].state = ThreadState::Running;
        let status = k.sys_send(client_cap, 9, 0, 0);
        assert_eq!(status, OK);
        assert_eq!(k.threads.slots[client].state, ThreadState::BlockedSend);
        k.cpu.current = _server;
        k.threads.slots[_server].state = ThreadState::Running;
        let status = k.sys_recv(1);
        assert_eq!(status, OK);
        assert_eq!(k.threads.slots[_server].ipc_msg.words[0], 9);
        assert_eq!(k.threads.slots[client].state, ThreadState::Ready);
        let _ = SYS_SEND;
        let _ = SYS_RECV;
    }
}
