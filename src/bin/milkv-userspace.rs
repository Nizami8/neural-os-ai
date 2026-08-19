#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

mod scheduler;
mod trap;
mod neural;
mod adaptive;

use adaptive::AdaptiveScheduler;
use neural::TaskClass;

const SEPARATOR: &str = "═══════════════════════════════════════";

#[cfg(test)]
fn main() {}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    println!("\n🤖 Neural OS v0.9 - Milk-V Duo 256M Optimized");
    println!("{}", SEPARATOR);

    let mut adaptive = AdaptiveScheduler::new();

    println!("📊 Initializing 2-core scheduler...");
    println!("   Memory: 256 MB");
    println!("   History: 32 samples (optimized)");
    
    // Добавляем 2 задачи (max для 256MB)
    adaptive.add_task(1, task1, TaskClass::RealTime);
    adaptive.add_task(2, task2, TaskClass::Batch);

    println!("🧠 MLP Network: 7→8→1");
    println!("   Momentum SGD enabled");
    
    println!("⏱️  Starting 10ms quantum timers...");
    println!("{}\n", SEPARATOR);

    // Эмулируем scheduler в цикле
    let mut tick = 0;
    let mut stat_counter = 0;
    
    loop {
        // Симулируем tick каждую миллисекунду
        std::thread::sleep(std::time::Duration::from_millis(1));
        tick += 1;
        stat_counter += 1;
        
        // Каждые 500 тиков выводим статистику
        if stat_counter >= 500 {
            stat_counter = 0;
            adaptive.collect_statistics();
            
            println!("\n📊 Statistics (tick={}):", tick);
            println!("   Context Switches: {}", adaptive.stats.total_context_switches);
            println!("   Avg Wait Time: {:.2} ticks", adaptive.stats.avg_wait_time);
            println!("   Fairness Index: {:.2}", adaptive.stats.fairness_index);
            
            let weights = adaptive.get_neural_weights();
            print!("   Neural Weights: ");
            for (i, w) in weights.iter().take(4).enumerate() {
                if i > 0 { print!(", "); }
                print!("{:.3}", w);
            }
            println!();
            println!("{}\n", SEPARATOR);
        }
    }
}

/// Бесконечная задача: печатает свой счетчик и жжет busy_cycles тактов
fn counter_task(id: usize, busy_cycles: usize) -> ! {
    let mut counter = 0u64;
    loop {
        print!("[T{}:{}]", id, counter);
        counter = counter.wrapping_add(1);

        for _ in 0..busy_cycles {
            unsafe { core::arch::asm!("nop") };
        }
    }
}

fn task1() {
    counter_task(1, 50)
}

fn task2() {
    counter_task(2, 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_footprint() {
        assert!(std::mem::size_of::<AdaptiveScheduler>() < 1024 * 256);
    }
}
