# Machine-mode trap vector with full context save/restore for preemptive
# multitasking. On every trap it saves the running task's entire register file
# into that task's TrapFrame (pointed to by the global CURRENT_TF), runs the
# Rust handler on the kernel stack (which may pick a different task and update
# CURRENT_TF), then restores the now-current task's TrapFrame and mret's into it.
#
# TrapFrame layout (see src/trapframe.rs):
#   regs[i] at offset i*8  (x1..x31; x0 unused)
#   mepc    at offset 256
#   mstatus at offset 264

.section .text
.globl trap_vector
.align 4
trap_vector:
    # a0 becomes the TrapFrame base pointer; stash the task's real a0 in mscratch.
    csrw    mscratch, a0
    la      a0, CURRENT_TF
    ld      a0, 0(a0)              # a0 = *CURRENT_TF (running task's frame)

    sd      x1,  1*8(a0)
    sd      x2,  2*8(a0)
    sd      x3,  3*8(a0)
    sd      x4,  4*8(a0)
    sd      x5,  5*8(a0)           # t0 saved; free to use below
    sd      x6,  6*8(a0)
    sd      x7,  7*8(a0)
    sd      x8,  8*8(a0)
    sd      x9,  9*8(a0)
    csrr    t0, mscratch
    sd      t0, 10*8(a0)           # x10 = task's original a0
    sd      x11, 11*8(a0)
    sd      x12, 12*8(a0)
    sd      x13, 13*8(a0)
    sd      x14, 14*8(a0)
    sd      x15, 15*8(a0)
    sd      x16, 16*8(a0)
    sd      x17, 17*8(a0)
    sd      x18, 18*8(a0)
    sd      x19, 19*8(a0)
    sd      x20, 20*8(a0)
    sd      x21, 21*8(a0)
    sd      x22, 22*8(a0)
    sd      x23, 23*8(a0)
    sd      x24, 24*8(a0)
    sd      x25, 25*8(a0)
    sd      x26, 26*8(a0)
    sd      x27, 27*8(a0)
    sd      x28, 28*8(a0)
    sd      x29, 29*8(a0)
    sd      x30, 30*8(a0)
    sd      x31, 31*8(a0)

    csrr    t0, mepc
    sd      t0, 256(a0)
    csrr    t0, mstatus
    sd      t0, 264(a0)

    # Run the handler on a dedicated kernel stack.
    la      sp, _stack_top
    call    rust_trap_handler

    # Reload (possibly switched) current frame and restore into it.
    la      a0, CURRENT_TF
    ld      a0, 0(a0)
    j       __restore_frame

# Bootstrap entry: launch the first task using CURRENT_TF.
.globl start_scheduling
start_scheduling:
    la      a0, CURRENT_TF
    ld      a0, 0(a0)
    # fall through

# Restore the full register file from the TrapFrame in a0 and mret into it.
__restore_frame:
    ld      t0, 264(a0)
    csrw    mstatus, t0
    ld      t0, 256(a0)
    csrw    mepc, t0

    ld      x1,  1*8(a0)
    ld      x2,  2*8(a0)
    ld      x3,  3*8(a0)
    ld      x4,  4*8(a0)
    ld      x5,  5*8(a0)
    ld      x6,  6*8(a0)
    ld      x7,  7*8(a0)
    ld      x8,  8*8(a0)
    ld      x9,  9*8(a0)
    ld      x11, 11*8(a0)
    ld      x12, 12*8(a0)
    ld      x13, 13*8(a0)
    ld      x14, 14*8(a0)
    ld      x15, 15*8(a0)
    ld      x16, 16*8(a0)
    ld      x17, 17*8(a0)
    ld      x18, 18*8(a0)
    ld      x19, 19*8(a0)
    ld      x20, 20*8(a0)
    ld      x21, 21*8(a0)
    ld      x22, 22*8(a0)
    ld      x23, 23*8(a0)
    ld      x24, 24*8(a0)
    ld      x25, 25*8(a0)
    ld      x26, 26*8(a0)
    ld      x27, 27*8(a0)
    ld      x28, 28*8(a0)
    ld      x29, 29*8(a0)
    ld      x30, 30*8(a0)
    ld      x31, 31*8(a0)
    ld      x10, 10*8(a0)          # restore a0 last (it was the base pointer)

    mret
