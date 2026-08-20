/// Full RISC-V (RV64) machine-mode trap frame for preemptive multitasking.
///
/// Every task owns one of these. On a timer interrupt the assembly trap vector
/// (`trap.s`) saves the running task's entire integer register file plus `mepc`
/// and `mstatus` here, the scheduler picks the next task, and the vector then
/// restores that task's frame and `mret`s into it. This is the per-task
/// `TrapFrame` half of the classic two-level (TrapFrame + Context) design.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrapFrame {
    /// x0..x31 (x0 is unused/hardwired zero; kept for simple index = xN offsets).
    pub regs: [usize; 32],
    /// Program counter to resume at (restored into mepc).
    pub mepc: usize,
    /// Saved mstatus (privilege, prior-interrupt-enable, FPU state).
    pub mstatus: usize,
}

impl TrapFrame {
    pub const fn zero() -> Self {
        TrapFrame {
            regs: [0; 32],
            mepc: 0,
            mstatus: 0,
        }
    }
}

// mstatus bits used to launch a task via mret.
const MSTATUS_MPP_M: usize = 3 << 11; // previous privilege = Machine
const MSTATUS_MPIE: usize = 1 << 7; // enable interrupts after mret
const MSTATUS_FS_DIRTY: usize = 3 << 13; // FPU on (tasks/handler use f32)

/// Initial mstatus for a freshly created task: return to M-mode with interrupts
/// enabled and the FPU active.
pub const MSTATUS_INIT: usize = MSTATUS_MPP_M | MSTATUS_MPIE | MSTATUS_FS_DIRTY;
