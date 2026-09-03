//! Milk-V Duo userspace demo.
//!
//! Runs the shared neural scheduler library under Linux (glibc) on the board.
//! This is not the bare-metal kernel — it validates that the MLP + storage
//! modules work on the Duo's RISC-V Linux userspace.

use neural_os::neural::{NeuralScheduler, TaskClass, TaskMetrics};
use neural_os::storage::PersistentStorage;
use std::thread;
use std::time::Duration;

fn main() {
    println!("\n🤖 Neural OS v0.9 - Milk-V Duo userspace");
    println!("═══════════════════════════════════════");

    let mut net = NeuralScheduler::new();
    let mut storage = PersistentStorage::new();

    if let Some((h, o)) = storage.load_weights() {
        net.import_weights(&h, &o);
        println!("💾 Restored persisted weights");
    } else {
        println!("💾 Fresh network (no persisted weights)");
    }

    println!("🧠 MLP 7→8→1 (Q16.16 fixed-point)");
    println!("   Classes: {:?} / {:?}", TaskClass::RealTime, TaskClass::Batch);
    println!("⏱️  Running online learning demo...\n");

    let mut tick = 0u32;
    loop {
        tick = tick.wrapping_add(1);

        for (id, wait) in [(1u32, 80u32), (2, 20), (3, 120)] {
            let mut m = TaskMetrics::new(id as usize);
            m.execution_time = 100 + (tick % 50);
            m.wait_time = wait + (tick % 10);
            m.ticks_since_run = m.wait_time;
            let p = net.predict_priority(&m);
            let target = if m.wait_time > 50 { 0.9 } else { 0.5 };
            net.learn(&m, target);
            if tick % 200 == 0 {
                println!("[tick={tick}] task={id} priority={p:.3} target={target:.1}");
            }
        }

        if tick % 500 == 0 {
            let (h, o) = net.export_weights();
            storage.save_weights(&h, &o);
            let w = net.get_output_weights();
            println!(
                "📊 persisted weights: {:.3}, {:.3}, {:.3}, {:.3}",
                w[0], w[1], w[2], w[3]
            );
            println!("═══════════════════════════════════════\n");
        }

        thread::sleep(Duration::from_millis(2));
    }
}
