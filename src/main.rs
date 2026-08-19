#![no_std]
#![no_main]

mod scheduler;
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

fn print_labeled_number(label: &str, value: usize, suffix: &str) {
    puts(label);
    print_number(value);
    puts(suffix);
}

fn print_labeled_float(label: &str, value: f32, suffix: &str) {
    puts(label);
    print_float(value, 2);
    puts(suffix);
}

/// Выводит count весов через запятую
fn print_weights(weights: &[f32], count: usize) {
    for i in 0..count {
        if i > 0 {
            puts(", ");
        }
        print_float(weights[i], 2);
    }
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

/// Бесконечная задача: печатает свой счетчик и жжет busy_cycles тактов
fn counter_task(id: usize, class_tag: &str, busy_cycles: usize) -> ! {
    let mut counter = 0u64;
    loop {
        puts("[T");
        print_number(id);
        putc(b':');
        print_number(counter as usize);
        putc(b'|');
        puts(class_tag);
        puts("] ");
        counter = counter.wrapping_add(1);

        for _ in 0..busy_cycles {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

fn task1() {
    counter_task(1, "RT", 50)
}

fn task2() {
    counter_task(2, "IO", 75)
}

fn task3() {
    counter_task(3, "BG", 100)
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
        print_weights(&weights, 4);
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

        init_timer();
    }

    // Главный loop с периодическим мониторингом
    let mut stat_counter = 0;
    loop {
        unsafe {
            core::arch::asm!("wfi");
            
            stat_counter += 1;
            
            // Каждые 1000 тиков выводим статистику
            if stat_counter >= 1000 {
                stat_counter = 0;
                
                if let Some(ref mut adaptive) = ADAPTIVE_SCHED {
                    adaptive.collect_statistics();
                    
                    puts("\n");
                    puts("📊 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ STATISTICS ━━━━━━━━\n");
                    print_labeled_number(
                        "   Context Switches: ",
                        adaptive.stats.total_context_switches as usize,
                        "\n",
                    );
                    print_labeled_float("   Avg Wait Time: ", adaptive.stats.avg_wait_time, " ticks\n");
                    print_labeled_float(
                        "   Fairness Index: ",
                        adaptive.stats.fairness_index,
                        " (1.0 = perfect)\n",
                    );

                    puts("   Neural Output Weights: ");
                    print_weights(&adaptive.get_neural_weights(), 4);
                    puts("\n");
                    
                    puts("────────────────────────────────────────────────────────\n\n");
                }
            }
        }
    }
}
