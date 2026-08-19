#![no_std]
#![no_main]

mod scheduler;
mod trap;
mod neural;
mod adaptive;
mod storage;

use scheduler::Scheduler;
use trap::init_timer;
use adaptive::{AdaptiveScheduler, SchedulerError, SchedulingStrategy};
use neural::{NeuralError, TaskClass};
use storage::{PersistentStorage, StorageError};
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

pub(crate) fn puts(s: &str) {
    for b in s.bytes() {
        putc(b);
    }
}

pub(crate) fn print_number(n: usize) {
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

/// `f32::abs` живет в std, которого в ядре нет.
fn abs_f32(value: f32) -> f32 {
    if value < 0.0 { -value } else { value }
}

fn print_float(f: f32, decimals: usize) {
    let int_part = f as i32;
    if int_part < 0 {
        putc(b'-');
    }
    print_number(int_part.unsigned_abs() as usize);
    putc(b'.');
    
    let frac = ((abs_f32(f) - (int_part.unsigned_abs() as f32)) * 100.0) as usize;
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

/// Сколько ошибок планировщика уже было отчитано в UART.
static SCHED_ERRORS_REPORTED: AtomicUsize = AtomicUsize::new(0);

/// Сколько первых ошибок печатаем подробно: обработчик таймера срабатывает
/// 100 раз в секунду, и бесконечный лог сам стал бы проблемой.
const MAX_REPORTED_SCHED_ERRORS: usize = 8;

fn neural_error_name(error: NeuralError) -> &'static str {
    match error {
        NeuralError::NonFiniteTarget => "non-finite target",
        NeuralError::NonFiniteOutput => "non-finite output",
        NeuralError::NonFiniteGradient => "non-finite gradient (update rejected)",
    }
}

fn storage_error_name(error: StorageError) -> &'static str {
    match error {
        StorageError::BufferTooSmall { .. } => "storage buffer too small",
        StorageError::Empty => "storage empty",
        StorageError::Corrupted { .. } => "storage corrupted",
    }
}

/// Выводит ошибку планировщика в UART с ограничением частоты.
pub(crate) fn report_scheduler_error(context: &str, error: SchedulerError) {
    let reported = SCHED_ERRORS_REPORTED.fetch_add(1, Ordering::Relaxed);
    if reported > MAX_REPORTED_SCHED_ERRORS {
        return;
    }

    puts("\n⚠️  ");
    puts(context);
    puts(": ");

    match error {
        SchedulerError::InvalidTaskId(id) => {
            puts("invalid task id ");
            print_number(id);
        }
        SchedulerError::MissingTask(idx) => {
            puts("task slot ");
            print_number(idx);
            puts(" is empty");
        }
        SchedulerError::NoRunnableTask => puts("no runnable task"),
        SchedulerError::Neural(neural_error) => {
            puts("neural failure: ");
            puts(neural_error_name(neural_error));
        }
    }

    puts("\n");

    if reported == MAX_REPORTED_SCHED_ERRORS {
        puts("⚠️  further scheduler errors suppressed, see statistics counters\n");
    }
}

fn report_scheduler_error_count() {
    let total = SCHED_ERRORS_REPORTED.load(Ordering::Relaxed);
    puts("   Scheduler Errors: ");
    print_number(total);
    puts("\n");
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    puts("\nPANIC");
    if let Some(location) = info.location() {
        puts(" at ");
        puts(location.file());
        puts(":");
        print_number(location.line() as usize);
    }
    puts("\n");
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

        for _ in 0..50 {
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

        for _ in 0..75 {
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

        for _ in 0..100 {
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

        // Восстанавливаем веса с прошлого запуска, если они есть и валидны.
        match storage.load_weights() {
            Ok((hidden_weights, output_weights)) => {
                match adaptive.neural.load_weights(&hidden_weights, &output_weights) {
                    Ok(()) => puts("💾 Restored neural weights from persistent storage\n"),
                    Err(error) => {
                        puts("⚠️  Rejected stored weights: ");
                        puts(neural_error_name(error));
                        puts(" (starting from defaults)\n");
                    }
                }
            }
            Err(StorageError::Empty) => {
                puts("💾 No stored weights yet, starting from defaults\n");
            }
            Err(error) => {
                puts("⚠️  Could not load stored weights: ");
                puts(storage_error_name(error));
                puts(" (starting from defaults)\n");
            }
        }

        STORAGE = Some(storage);

        puts("📊 Initializing advanced scheduler...\n");
        
        // Добавляем задачи с классами
        if let Err(error) = adaptive.add_task(1, task1, TaskClass::RealTime) {
            report_scheduler_error("add_task(1)", error);
        }
        if let Err(error) = adaptive.add_task(2, task2, TaskClass::Interactive) {
            report_scheduler_error("add_task(2)", error);
        }
        if let Err(error) = adaptive.add_task(3, task3, TaskClass::Batch) {
            report_scheduler_error("add_task(3)", error);
        }

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
        if let Err(error) = adaptive.set_task_deadline(1, 200) {
            report_scheduler_error("set_task_deadline(1)", error);
        }
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

                    // Сохраняем выученные веса, чтобы они пережили перезагрузку.
                    if let Some(ref mut storage) = STORAGE {
                        if let Err(error) = storage.save_weights(
                            &adaptive.neural.hidden_weights,
                            &adaptive.neural.output_weights,
                        ) {
                            puts("⚠️  Failed to persist neural weights: ");
                            puts(storage_error_name(error));
                            puts("\n");
                        }
                    }
                    
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

                    puts("   Neural Errors: ");
                    print_number(adaptive.stats.neural_errors as usize);
                    puts("\n");

                    puts("   Unhandled Interrupts: ");
                    print_number(trap::unhandled_interrupt_count());
                    puts("\n");

                    report_scheduler_error_count();
                    
                    puts("────────────────────────────────────────────────────────\n\n");
                }
            }
        }
    }
}
