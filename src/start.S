.section .text.boot, "ax"
.globl _start
.globl _park

_start:
    csrr    t0, mhartid
    bnez    t0, _park

    la      sp, _stack_top

    la      t0, _bss_start
    la      t1, _bss_end
1:
    bgeu    t0, t1, 2f
    sd      zero, 0(t0)
    addi    t0, t0, 8
    j       1b
2:
    la      t0, trap_entry
    csrw    mtvec, t0

    csrw    mie, zero
    csrci   mstatus, 8

    call    rust_main

_park:
    wfi
    j       _park
