# Boot entry for the Neural OS bare-metal RISC-V kernel.
# QEMU (-bios none) enters here in M-mode with interrupts disabled.

.section .text.entry
.globl _start
_start:
    # Set up the kernel stack.
    la      sp, _stack_top

    # Enable the FPU (mstatus.FS = Dirty). The neural network uses f32, so the
    # floating-point unit must be turned on before any Rust code runs.
    li      t0, (3 << 13)
    csrs    mstatus, t0

    # Jump into Rust. rust_main never returns.
    call    rust_main

# Safety net: spin if rust_main ever returns.
1:
    wfi
    j       1b
