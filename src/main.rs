#![no_std]
#![no_main]

mod scheduler;
mod trap;
mod neural;
mod adaptive;

use scheduler::Scheduler;
use trap::init_timer;
use adaptive::AdaptiveScheduler;
use core::sync::atomic::{AtomicUsize, Ordering};

const UART: *mut u8 = 0x10000000 as *mut u8;

pub static mut SCHED: Scheduler = Scheduler::new();
pub static mut ADAPTIVE_SCHED: Option<AdaptiveScheduler> = None;
static TICK_COUNTER: AtomicUsize = AtomicUsize::new(0);
static USE_AI: AtomicUsize = AtomicUsize::new(0); // флаг для включения AI

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
    let int_part = f as usize;
    let frac_part = ((f - int_part as f32) * 100.0) as usize;

    print_number(int_part);
    putc(b'.');
    if frac_part < 10 {
        putc(b'0');
    }
    print_number(frac_part);
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

fn task1() {
    let mut counter = 0u64;
    loop {
        puts("[T1:");
        print_number(counter as usize);
        puts("] ");
        counter += 1;

        for _ in 0..100 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

fn task2() {
    let mut counter = 0u64;
    loop {
        puts("[T2:");
        print_number(counter as usize);
        puts("] ");
        counter += 1;

        for _ in 0..100 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

fn task3() {
    let mut counter = 0u64;
    loop {
        puts("[T3:");
        print_number(counter as usize);
        puts("] ");
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
    puts("╔═══════════════════════════════════════════════╗\n");
    puts("║   🤖 Neural OS v0.8 - AI Learning Kernel    ║\n");
    puts("║      Online-Learning Adaptive Scheduler      ║\n");
    puts("╚═══════════════════════════════════════════════╝\n");
    puts("\n");

    unsafe {
        // Инициализируем адаптивный планировщик
        let mut adaptive = AdaptiveScheduler::new();

        puts("📊 Initializing AI-powered scheduler...\n");
        adaptive.add_task(1, task1);
        adaptive.add_task(2, task2);
        adaptive.add_task(3, task3);

        puts("🧠 Neural network initialized\n");
        puts("   Initial weights: ");
        let weights = adaptive.get_neural_weights();
        print_float(weights[0], 2);
        puts(", ");
        print_float(weights[1], 2);
        puts(", ");
        print_float(weights[2], 2);
        puts(", ");
        print_float(weights[3], 2);
        puts("\n");

        puts("⏱️  Starting 10ms quantum timers...\n");
        puts("────────────────────────────────────────────\n\n");

        ADAPTIVE_SCHED = Some(adaptive);
        USE_AI.store(1, Ordering::Relaxed);

        init_timer();
    }

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
