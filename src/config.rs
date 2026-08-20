//! Compile-time kernel limits. Sized for QEMU virt and Milk-V Duo 256M.

pub const MAX_PROCESSES: usize = 8;
pub const MAX_THREADS: usize = 16;
pub const MAX_ENDPOINTS: usize = 16;
pub const MAX_CAPS: usize = 16;
pub const WAIT_QUEUE: usize = 8;

pub const KERNEL_STACK_SIZE: usize = 4096;
pub const DEFAULT_QUANTUM: usize = 3;

pub const UART_BASE: usize = 0x1000_0000;
pub const CLINT_BASE: usize = 0x0200_0000;
pub const CLINT_MTIMECMP: usize = CLINT_BASE + 0x4000;
pub const CLINT_MTIME: usize = CLINT_BASE + 0xBFF8;

/// QEMU virt timebase frequency.
pub const TIMEBASE_FREQ: u64 = 10_000_000;
/// 100 Hz preemption.
pub const TIMER_HZ: u64 = 100;
pub const TIMER_INTERVAL: u64 = TIMEBASE_FREQ / TIMER_HZ;
