# TrapFrame offsets — keep in sync with src/trapframe.rs
# ra=0 sp=8 gp=16 tp=24 t0=32 t1=40 t2=48 s0=56 s1=64
# a0=72 a1=80 a2=88 a3=96 a4=104 a5=112 a6=120 a7=128
# s2=136 s3=144 s4=152 s5=160 s6=168 s7=176 s8=184 s9=192 s10=200 s11=208
# t3=216 t4=224 t5=232 t6=240 mepc=248 mstatus=256 mcause=264 mtval=272

.section .text
.globl trap_entry
.globl trap_return
.align 4

trap_entry:
    csrrw   t6, mscratch, t6
    beqz    t6, trap_no_frame

    sd      ra,   0(t6)
    sd      sp,   8(t6)
    sd      gp,  16(t6)
    sd      tp,  24(t6)
    sd      t0,  32(t6)
    sd      t1,  40(t6)
    sd      t2,  48(t6)
    sd      s0,  56(t6)
    sd      s1,  64(t6)
    sd      a0,  72(t6)
    sd      a1,  80(t6)
    sd      a2,  88(t6)
    sd      a3,  96(t6)
    sd      a4, 104(t6)
    sd      a5, 112(t6)
    sd      a6, 120(t6)
    sd      a7, 128(t6)
    sd      s2, 136(t6)
    sd      s3, 144(t6)
    sd      s4, 152(t6)
    sd      s5, 160(t6)
    sd      s6, 168(t6)
    sd      s7, 176(t6)
    sd      s8, 184(t6)
    sd      s9, 192(t6)
    sd      s10,200(t6)
    sd      s11,208(t6)
    sd      t3, 216(t6)
    sd      t4, 224(t6)
    sd      t5, 232(t6)

    csrr    t5, mscratch
    sd      t5, 240(t6)

    csrr    t5, mepc
    sd      t5, 248(t6)
    csrr    t5, mstatus
    sd      t5, 256(t6)
    csrr    t5, mcause
    sd      t5, 264(t6)
    csrr    t5, mtval
    sd      t5, 272(t6)

    csrw    mscratch, t6
    mv      a0, t6
    la      sp, _trap_stack_top
    call    rust_trap_handler
    j       trap_return

trap_no_frame:
    csrrw   t6, mscratch, t6
    j       _park

trap_return:
    mv      t6, a0
    csrw    mscratch, t6

    ld      t5, 248(t6)
    csrw    mepc, t5
    ld      t5, 256(t6)
    csrw    mstatus, t5

    ld      ra,   0(t6)
    ld      sp,   8(t6)
    ld      gp,  16(t6)
    ld      tp,  24(t6)
    ld      t0,  32(t6)
    ld      t1,  40(t6)
    ld      t2,  48(t6)
    ld      s0,  56(t6)
    ld      s1,  64(t6)
    ld      a0,  72(t6)
    ld      a1,  80(t6)
    ld      a2,  88(t6)
    ld      a3,  96(t6)
    ld      a4, 104(t6)
    ld      a5, 112(t6)
    ld      a6, 120(t6)
    ld      a7, 128(t6)
    ld      s2, 136(t6)
    ld      s3, 144(t6)
    ld      s4, 152(t6)
    ld      s5, 160(t6)
    ld      s6, 168(t6)
    ld      s7, 176(t6)
    ld      s8, 184(t6)
    ld      s9, 192(t6)
    ld      s10,200(t6)
    ld      s11,208(t6)
    ld      t3, 216(t6)
    ld      t4, 224(t6)
    ld      t5, 232(t6)
    ld      t6, 240(t6)
    mret
