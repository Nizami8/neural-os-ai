//! Linux/Milk-V userspace simulation of Neural OS scheduling + IPC.

use neural_os::adaptive::{AdaptiveScheduler, RewardSignal, SchedulingStrategy};
use neural_os::capability::{Capability, CapabilityTable, RIGHT_RECV, RIGHT_SEND};
use neural_os::ipc::{Endpoint, Message};
use neural_os::kernel::Kernel;
use neural_os::neural::TaskClass;
use neural_os::syscall::OK;
use neural_os::thread::ThreadState;

fn dummy() {}

fn main() {
    println!("\n🤖 Neural OS v1.0-alpha — Milk-V / host simulation");
    println!("═══════════════════════════════════════");

    let mut adaptive = AdaptiveScheduler::new();
    adaptive.add_task(1, dummy, TaskClass::RealTime);
    adaptive.add_task(2, dummy, TaskClass::Batch);
    adaptive.strategy = SchedulingStrategy::LoadBalanced;
    adaptive.set_task_deadline(1, 200);
    adaptive.learn_with_reward(RewardSignal {
        task_id: 1,
        reward: 1.0,
    });
    adaptive.collect_statistics();

    println!("MLP 7→8→1  fairness={:.2}", adaptive.stats.fairness_index);
    let w = adaptive.get_neural_weights();
    println!("neural out[0..4]: {:.3} {:.3} {:.3} {:.3}", w[0], w[1], w[2], w[3]);

    let mut kernel = Kernel::new();
    let (server_pid, server_tid) = kernel
        .spawn(dummy as usize, TaskClass::Interactive, 8)
        .unwrap();
    let (client_pid, client_tid) = kernel
        .spawn(dummy as usize, TaskClass::Batch, 4)
        .unwrap();
    let ep = kernel.endpoints.alloc().unwrap();
    let (_sc, cc) = kernel
        .install_endpoint_caps(ep, server_pid, client_pid)
        .unwrap();

    kernel.cpu.current = server_tid;
    kernel.threads.slots[server_tid].state = ThreadState::Running;
    assert_eq!(kernel.sys_recv(1), OK);
    assert_eq!(
        kernel.threads.slots[server_tid].state,
        ThreadState::BlockedRecv
    );

    kernel.cpu.current = client_tid;
    kernel.threads.slots[client_tid].state = ThreadState::Running;
    assert_eq!(kernel.sys_send(cc, 42, 1, 0), OK);
    assert_eq!(kernel.threads.slots[server_tid].ipc_msg.words[0], 42);

    let mut table = CapabilityTable::new();
    table
        .insert(Capability::endpoint(1, 1, RIGHT_RECV, 1))
        .unwrap();
    assert!(table.lookup_endpoint(1, RIGHT_SEND, 1, 0).is_err());

    let mut ep_obj = Endpoint::empty();
    ep_obj.activate();
    let _ = Message::new(client_tid, 1, 2, 3);

    println!("IPC rendezvous: ok");
    println!("capability deny-without-SEND: ok");
    println!("═══════════════════════════════════════");
}
