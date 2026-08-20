#![no_std]
#![no_main]

use neural_os::capability::RIGHT_SEND;
use neural_os::console;
use neural_os::kernel::KERNEL;
use neural_os::neural::TaskClass;
use neural_os::syscall::{
    SYS_GETPID, SYS_GETTID, SYS_PRINT, SYS_RECV, SYS_SEND, SYS_YIELD,
};
use neural_os::timer;
use neural_os::trap::trap_return;

fn ecall(nr: usize, mut a0: usize, a1: usize, a2: usize, a3: usize) -> usize {
    unsafe {
        core::arch::asm!(
            "ecall",
            inout("a0") a0,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("a7") nr,
            options(nostack)
        );
    }
    a0
}

fn sys_yield() {
    let _ = ecall(SYS_YIELD, 0, 0, 0, 0);
}

fn sys_print(s: &str) {
    let _ = ecall(SYS_PRINT, s.as_ptr() as usize, s.len(), 0, 0);
}

fn sys_send(cap: usize, w0: usize, w1: usize, w2: usize) -> usize {
    ecall(SYS_SEND, cap, w0, w1, w2)
}

fn sys_recv(cap: usize) -> (usize, usize, usize, usize) {
    let a0: usize;
    let a1: usize;
    let a2: usize;
    let a3: usize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inout("a0") cap => a0,
            lateout("a1") a1,
            lateout("a2") a2,
            lateout("a3") a3,
            in("a7") SYS_RECV,
            options(nostack)
        );
    }
    (a0, a1, a2, a3)
}

fn idle_thread() -> ! {
    loop {
        unsafe { core::arch::asm!("wfi") }
        sys_yield();
    }
}

fn server_thread() -> ! {
    sys_print("[server] recv on cap 1\n");
    loop {
        let (st, w0, w1, w2) = sys_recv(1);
        if st == 0 {
            sys_print("[server] got ");
            console::write_usize(w0);
            sys_print(" from ");
            console::write_usize(w1);
            sys_print(" seq=");
            console::write_usize(w2);
            sys_print("\n");
        } else {
            sys_print("[server] recv err\n");
            sys_yield();
        }
    }
}

fn client_thread() -> ! {
    sys_print("[client] send on cap 1\n");
    let mut seq = 0usize;
    loop {
        let pid = ecall(SYS_GETPID, 0, 0, 0, 0);
        let tid = ecall(SYS_GETTID, 0, 0, 0, 0);
        let st = sys_send(1, 42 + seq, pid, seq);
        if st == 0 {
            sys_print("[client] sent seq=");
            console::write_usize(seq);
            sys_print(" tid=");
            console::write_usize(tid);
            sys_print("\n");
        } else {
            sys_print("[client] send err=");
            console::write_usize(st);
            sys_print("\n");
        }
        seq = seq.wrapping_add(1);
        for _ in 0..4000 {
            unsafe { core::arch::asm!("nop") }
        }
        sys_yield();
    }
}

fn denied_thread() -> ! {
    // Only RIGHT_RECV was granted; send must be rejected.
    let st = sys_send(1, 99, 0, 0);
    sys_print("[denied] send status=");
    if st == neural_os::syscall::ERR_DENIED {
        sys_print("DENIED\n");
    } else {
        console::write_usize(st);
        sys_print("\n");
    }
    loop {
        sys_yield();
        unsafe { core::arch::asm!("wfi") }
    }
}

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    console::write_str("\n");
    console::write_str("===========================================================\n");
    console::write_str(" Neural OS v1.0-alpha  Stage 4: IPC + Capabilities\n");
    console::write_str("  * TrapFrame context switch + preemptive RR\n");
    console::write_str("  * Process/Thread + syscalls\n");
    console::write_str("  * Endpoint rendezvous (blocking send/recv)\n");
    console::write_str("  * Capability rights + generation checks\n");
    console::write_str("===========================================================\n\n");

    unsafe {
        #[allow(static_mut_refs)]
        let k = &mut KERNEL;

        let (_idle_pid, idle_tid) = k
            .spawn(idle_thread as usize, TaskClass::Batch, 1)
            .expect("idle");
        let (server_pid, _server_tid) = k
            .spawn(server_thread as usize, TaskClass::Interactive, 8)
            .expect("server");
        let (client_pid, _client_tid) = k
            .spawn(client_thread as usize, TaskClass::Interactive, 8)
            .expect("client");
        let (denied_pid, _denied_tid) = k
            .spawn(denied_thread as usize, TaskClass::Batch, 2)
            .expect("denied");

        let ep = k.endpoints.alloc().expect("endpoint");
        k.install_endpoint_caps(ep, server_pid, client_pid)
            .expect("caps");

        let gen = k.endpoints.slots[ep].generation;
        k.processes.slots[denied_pid]
            .caps
            .insert(neural_os::capability::Capability::endpoint(
                ep as u16,
                gen,
                neural_os::capability::RIGHT_RECV,
                denied_pid as u16,
            ))
            .expect("denied cap");

        let _ = RIGHT_SEND;

        console::write_str("endpoint=");
        console::write_usize(ep);
        console::write_str(" server_pid=");
        console::write_usize(server_pid);
        console::write_str(" client_pid=");
        console::write_usize(client_pid);
        console::write_str("\nstarting threads...\n\n");

        k.cpu.current = idle_tid;
        k.scheduler.current = idle_tid;
        k.threads.slots[idle_tid].state = neural_os::thread::ThreadState::Running;
        k.cpu.scheduler_enabled = true;

        timer::init();

        let tf = k.current_trapframe_ptr();
        trap_return(tf);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    console::write_str("PANIC: ");
    if let Some(loc) = info.location() {
        console::write_str(loc.file());
        console::write_str(":");
        console::write_usize(loc.line() as usize);
        console::write_str("\n");
    }
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
