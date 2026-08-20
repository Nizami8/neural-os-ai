#![no_std]
#![no_main]

mod scheduler;
mod trapframe;
mod trap;
mod neural;
mod adaptive;
mod storage;

use scheduler::Scheduler;
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

// f32 remainder (`%`) lowers to fmodf, which core does not provide on bare metal.
#[no_mangle]
pub extern "C" fn fmodf(x: f32, y: f32) -> f32 {
    if y == 0.0 {
        return f32::NAN;
    }
    let q = (x / y) as i32; // truncate toward zero
    x - (q as f32) * y
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    puts("PANIC!\n");
    loop {}
}

// ====== ЗАДАЧИ ======

fn task1() {
    let mut counter = 0u64;
    loop {
        puts("[T1:");
        print_number(counter as usize);
        puts("|RT] ");
        counter += 1;

        for _ in 0..2_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

fn task2() {
    let mut counter = 0u64;
    loop {
        puts("[T2:");
        print_number(counter as usize);
        puts("|IO] ");
        counter += 1;

        for _ in 0..2_600_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

fn task3() {
    let mut counter = 0u64;
    loop {
        puts("[T3:");
        print_number(counter as usize);
        puts("|BG] ");
        counter += 1;

        for _ in 0..3_200_000 {
            unsafe { core::arch::asm!("nop"); }
        }
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
        adaptive.add_task(1, task1, TaskClass::RealTime);      // жесткие deadline'ы
        adaptive.add_task(2, task2, TaskClass::Interactive);   // низкий latency
        adaptive.add_task(3, task3, TaskClass::Batch);         // фоновая работа

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
