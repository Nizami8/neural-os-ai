#![no_std]
#![no_main]

mod scheduler;
mod trapframe;
mod trap;
mod syscall;
mod ipc;
mod neural;
mod adaptive;
mod storage;

use scheduler::{Scheduler, TaskState};
use trap::init_timer;
use adaptive::{AdaptiveScheduler, SchedulingStrategy, RewardSignal};
use neural::TaskClass;
use storage::PersistentStorage;
use core::sync::atomic::{AtomicUsize, Ordering};

const UART: *mut u8 = 0x10000000 as *mut u8;

pub static mut SCHED: Scheduler = Scheduler::new();
pub static mut ADAPTIVE_SCHED: Option<AdaptiveScheduler> = None;
pub static mut STORAGE: Option<PersistentStorage> = None;
static TICK_COUNTER: AtomicUsize = AtomicUsize::new(0);
static USE_AI: AtomicUsize = AtomicUsize::new(0);

fn putc(c: u8) {
    unsafe { core::ptr::write_volatile(UART, c); }
}

// NS16550 UART receive path (QEMU virt): LSR at offset 5, bit 0 = data ready;
// RBR at offset 0 (same address as THR, but reads return received bytes).
fn uart_can_read() -> bool {
    unsafe { core::ptr::read_volatile(UART.add(5)) & 1 != 0 }
}

fn uart_getc() -> u8 {
    unsafe { core::ptr::read_volatile(UART) }
}

fn puts(s: &str) {
    for b in s.bytes() {
        putc(b);
    }
}

fn print_number(n: usize) {
    if n == 0 {
        putc(b'0');
        return;
    }

    let mut digits = [0u8; 20];
    let mut i = 0;
    let mut num = n;

    while num > 0 && i < 20 {
        digits[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        putc(digits[i]);
    }
}

fn print_float(f: f32, decimals: usize) {
    let int_part = f as i32;
    if int_part < 0 {
        putc(b'-');
    }
    print_number(int_part.abs() as usize);
    putc(b'.');
    
    let frac = ((f.abs() - (int_part.abs() as f32)) * 100.0) as usize;
    if frac < 10 {
        putc(b'0');
    }
    print_number(frac);
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.add(i) = c as u8;
        i += 1;
    }
    s
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    puts("PANIC!\n");
    loop {}
}

// ====== ЗАДАЧИ ======

// Background compute tasks. They run silently (so the interactive shell stays
// readable) but keep consuming CPU, so `ps` shows their run time growing and the
// preemptive scheduler visibly juggling them.
fn task1() {
    let mut work = 0u64;
    loop {
        work = work.wrapping_add(1);
        core::hint::black_box(work);
        for _ in 0..300_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

fn task2() {
    let mut work = 0u64;
    loop {
        work = work.wrapping_add(1);
        core::hint::black_box(work);
        for _ in 0..500_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

fn task3() {
    let mut work = 0u64;
    loop {
        work = work.wrapping_add(1);
        core::hint::black_box(work);
        for _ in 0..700_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

// Interactive UART shell: reads a command line and asks the kernel (via
// syscalls) to report state. This realizes the project's original goal of
// interacting with the OS through a command line.
fn task7_shell() {
    syscall::sys_print("\n[shell] ready. commands: help ps stats nn\nneural-os> ");
    let mut buf = [0u8; 64];
    let mut len = 0usize;
    loop {
        if uart_can_read() {
            let c = uart_getc();
            match c {
                b'\r' | b'\n' => {
                    syscall::sys_print("\n");
                    shell_exec(&buf[..len]);
                    len = 0;
                    syscall::sys_print("neural-os> ");
                }
                0x08 | 0x7f => {
                    if len > 0 {
                        len -= 1;
                        syscall::sys_print("\x08 \x08");
                    }
                }
                _ => {
                    if len < buf.len() {
                        buf[len] = c;
                        len += 1;
                        let one = [c];
                        if let Ok(s) = core::str::from_utf8(&one) {
                            syscall::sys_print(s); // echo
                        }
                    }
                }
            }
        }
    }
}

fn shell_exec(cmd: &[u8]) {
    match cmd {
        b"help" => syscall::sys_print("commands: help, ps, stats, nn\n"),
        b"ps" => syscall::sys_ps(),
        b"stats" => syscall::sys_stats(),
        b"nn" => syscall::sys_nn(),
        b"" => {}
        _ => syscall::sys_print("unknown command (try: help)\n"),
    }
}

// Stage 3 demo task: interacts with the kernel purely through system calls
// (ecall), then terminates itself with SYS_EXIT to show the scheduler retiring
// a task while the others keep running.
fn task4_syscalls() {
    let pid = syscall::sys_getpid();
    syscall::sys_print("\n[SYSCALL] T4 started via ecall, pid=");
    syscall::sys_print_usize(pid);
    syscall::sys_print("\n");

    // T4 was granted no IPC capabilities, so this send must be denied.
    let rc = syscall::sys_send(DEMO_ENDPOINT, 999);
    if rc != 0 {
        syscall::sys_print("[SYSCALL] T4 SYS_SEND on ep0 DENIED (no capability), rc=");
        syscall::sys_print_usize(rc);
        syscall::sys_print("\n");
    }

    let mut n = 0u64;
    loop {
        syscall::sys_print("{T4:SYS_PRINT} ");
        n += 1;
        if n >= 5 {
            syscall::sys_print("\n[SYSCALL] T4 calling SYS_EXIT\n");
            syscall::sys_exit();
        }
        // Give up the CPU cooperatively, then burn a little time.
        syscall::sys_yield();
        for _ in 0..2_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

// Stage 4 demo: two tasks rendezvous over an IPC endpoint. The producer sends a
// counter, the consumer blocks on recv until it arrives, prints it, and loops.
const DEMO_ENDPOINT: usize = 0;

fn task5_ipc_producer() {
    let mut value = 0usize;
    loop {
        // Silent IPC so the interactive shell console stays clean; the effect is
        // observable via `ps` (the consumer sits Blocked between messages).
        let _ = syscall::sys_send(DEMO_ENDPOINT, value);
        value += 1;
        for _ in 0..8_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

fn task6_ipc_consumer() {
    loop {
        let _ = syscall::sys_recv(DEMO_ENDPOINT);
    }
}

// ====== ГЛАВНАЯ ФУНКЦИЯ ======

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    puts("\n");
    puts("╔═══════════════════════════════════════════════════════════╗\n");
    puts("║   🤖 Neural OS v0.9 - Full AI Orchestration Kernel     ║\n");
    puts("║   • MLP with Momentum SGD                               ║\n");
    puts("║   • Load Balancing + Fairness                           ║\n");
    puts("║   • Predictive Preemption + Q-Learning                 ║\n");
    puts("║   • Real-time Priority + Persistent Memory             ║\n");
    puts("╚═══════════════════════════════════════════════════════════╝\n");
    puts("\n");

    unsafe {
        let mut adaptive = AdaptiveScheduler::new();
        
        // Инициализируем storage для сохранения весов
        let storage = PersistentStorage::new();
        STORAGE = Some(storage);

        puts("📊 Initializing advanced scheduler...\n");
        
        // Добавляем задачи с классами
        adaptive.add_task(1, task1, TaskClass::RealTime);            // жесткие deadline'ы
        adaptive.add_task(2, task2, TaskClass::Interactive);         // низкий latency
        adaptive.add_task(3, task3, TaskClass::Batch);               // фоновая работа
        adaptive.add_task(4, task4_syscalls, TaskClass::Batch);      // демо системных вызовов
        adaptive.add_task(5, task5_ipc_producer, TaskClass::Batch);  // IPC producer
        adaptive.add_task(6, task6_ipc_consumer, TaskClass::Batch);  // IPC consumer
        adaptive.add_task(7, task7_shell, TaskClass::Interactive);   // interactive UART shell

        // Capabilities: only T5 may send and only T6 may recv on the endpoint.
        adaptive.grant_cap(5, DEMO_ENDPOINT, ipc::CAP_SEND);
        adaptive.grant_cap(6, DEMO_ENDPOINT, ipc::CAP_RECV);

        puts("🧠 Neural network initialized (MLP 7→8→1)\n");
        puts("   Architecture: Input(7) → Hidden(8, ReLU) → Output(1, Sigmoid)\n");
        puts("   Optimizer: SGD with Momentum (α=0.01, β=0.9)\n");
        
        puts("\n📈 Scheduling Strategy: LoadBalanced + Predictive\n");
        adaptive.strategy = SchedulingStrategy::LoadBalanced;
        
        puts("   Output weights: ");
        let weights = adaptive.get_neural_weights();
        for i in 0..4 {
            print_float(weights[i], 2);
            if i < 3 { puts(", "); }
        }
        puts("\n");

        puts("⏱️  Setting task deadlines...\n");
        adaptive.set_task_deadline(1, 200);  // Task 1: hard deadline через 200 тиков
        puts("   T1: 200 ticks (RealTime)\n");
        puts("   T2: unlimited (Interactive)\n");
        puts("   T3: unlimited (Batch)\n");

        puts("\n⏳ Starting with 10ms quantum timers...\n");
        puts("────────────────────────────────────────────────────────\n\n");

        ADAPTIVE_SCHED = Some(adaptive);
        USE_AI.store(1, Ordering::Relaxed);

        // Point the current-task pointer at the first task, arm the timer, and
        // hand control to the preemptive scheduler. Tasks now run for real and
        // are preempted by the machine timer; start_scheduling never returns.
        if let Some(ref mut a) = ADAPTIVE_SCHED {
            if let Some(ref mut t0) = a.base_scheduler.tasks[0] {
                trap::CURRENT_TF = &mut t0.tf as *mut _;
            }
        }
        init_timer();
        trap::start_scheduling();
    }
}

/// Print the task table (served by SYS_PS).
pub fn print_ps(adaptive: &adaptive::AdaptiveScheduler) {
    puts("\nPID  STATE     CLASS  CPU(ticks)\n");
    for (i, slot) in adaptive.base_scheduler.tasks.iter().enumerate() {
        if let Some(t) = slot {
            puts("  ");
            print_number(t.id);
            puts("  ");
            puts(match t.state {
                TaskState::Ready => "Ready    ",
                TaskState::Running => "Running  ",
                TaskState::Blocked => "Blocked  ",
                TaskState::Terminated => "Term     ",
            });
            puts(match adaptive.task_classes[i] {
                TaskClass::RealTime => "RT     ",
                TaskClass::Interactive => "IO     ",
                TaskClass::Batch => "BG     ",
            });
            print_number(adaptive.total_exec_time[i] as usize);
            puts("\n");
        }
    }
}

/// Print the neural network's output weights (served by SYS_NN).
pub fn print_nn(adaptive: &adaptive::AdaptiveScheduler) {
    puts("\nNeural output weights: ");
    let w = adaptive.get_neural_weights();
    for i in 0..8 {
        print_float(w[i], 2);
        if i < 7 {
            puts(", ");
        }
    }
    puts("\n");
}

/// Print a live statistics block. Called from the timer trap handler.
pub fn print_stats(adaptive: &adaptive::AdaptiveScheduler) {
    puts("\n");
    puts("📊 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ STATISTICS ━━━━━━━━\n");
    puts("   Context Switches: ");
    print_number(adaptive.stats.total_context_switches as usize);
    puts("\n");

    puts("   Avg Wait Time: ");
    print_float(adaptive.stats.avg_wait_time, 2);
    puts(" ticks\n");

    puts("   Fairness Index: ");
    print_float(adaptive.stats.fairness_index, 2);
    puts(" (1.0 = perfect)\n");

    puts("   Neural Output Weights: ");
    let w = adaptive.get_neural_weights();
    for i in 0..3 {
        print_float(w[i], 2);
        puts(", ");
    }
    print_float(w[3], 2);
    puts("\n");

    puts("────────────────────────────────────────────────────────\n\n");
}
