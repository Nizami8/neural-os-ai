//! Full GPR + CSR snapshot used on every trap and context switch.

#![allow(dead_code)]

/// Offsets must match `trap.S`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TrapFrame {
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub mepc: usize,
    pub mstatus: usize,
    pub mcause: usize,
    pub mtval: usize,
}

impl TrapFrame {
    pub const fn zero() -> Self {
        Self {
            ra: 0,
            sp: 0,
            gp: 0,
            tp: 0,
            t0: 0,
            t1: 0,
            t2: 0,
            s0: 0,
            s1: 0,
            a0: 0,
            a1: 0,
            a2: 0,
            a3: 0,
            a4: 0,
            a5: 0,
            a6: 0,
            a7: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
            t3: 0,
            t4: 0,
            t5: 0,
            t6: 0,
            mepc: 0,
            mstatus: 0,
            mcause: 0,
            mtval: 0,
        }
    }

    /// Machine-mode thread: MPP = M, MPIE set so `mret` re-enables IRQs.
    pub fn init_kernel(&mut self, entry: usize, stack_top: usize) {
        *self = Self::zero();
        self.mepc = entry;
        self.ra = entry;
        self.sp = stack_top;
        self.mstatus = (3 << 11) | (1 << 7);
    }

    pub fn init_user(&mut self, entry: usize, stack_top: usize) {
        *self = Self::zero();
        self.mepc = entry;
        self.sp = stack_top;
        self.mstatus = 1 << 7;
    }

    #[inline]
    pub fn pc(&self) -> usize {
        self.mepc
    }

    #[inline]
    pub fn set_pc(&mut self, pc: usize) {
        self.mepc = pc;
    }

    #[inline]
    pub fn advance_pc(&mut self) {
        self.mepc = self.mepc.wrapping_add(4);
    }

    #[inline]
    pub fn syscall_number(&self) -> usize {
        self.a7
    }

    #[inline]
    pub fn arg0(&self) -> usize {
        self.a0
    }

    #[inline]
    pub fn arg1(&self) -> usize {
        self.a1
    }

    #[inline]
    pub fn arg2(&self) -> usize {
        self.a2
    }

    #[inline]
    pub fn arg3(&self) -> usize {
        self.a3
    }

    #[inline]
    pub fn set_return(&mut self, value: usize) {
        self.a0 = value;
    }

    pub fn set_ipc_return(&mut self, status: usize, w0: usize, w1: usize, w2: usize, sender: usize) {
        self.a0 = status;
        self.a1 = w0;
        self.a2 = w1;
        self.a3 = w2;
        self.a6 = sender;
    }
}

const _: () = {
    assert!(core::mem::size_of::<TrapFrame>() == 280);
    assert!(core::mem::size_of::<TrapFrame>() % 8 == 0);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_init_sets_mpp_machine() {
        let mut tf = TrapFrame::zero();
        tf.init_kernel(0x8000_1000, 0x8001_0000);
        assert_eq!(tf.mepc, 0x8000_1000);
        assert_eq!(tf.sp, 0x8001_0000);
        assert_eq!((tf.mstatus >> 11) & 3, 3);
    }
}
