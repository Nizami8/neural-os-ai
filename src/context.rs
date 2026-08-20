//! Callee-saved kernel context. TrapFrame holds the rest of the GPRs.

#![allow(dead_code)]

use crate::trapframe::TrapFrame;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Context {
    pub ra: usize,
    pub sp: usize,
    pub s0: usize,
    pub s1: usize,
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
}

impl Context {
    pub const fn zero() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s0: 0,
            s1: 0,
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
        }
    }

    pub fn init(&mut self, entry: usize, stack_top: usize) {
        *self = Self::zero();
        self.ra = entry;
        self.sp = stack_top;
    }
}

pub struct ExecutionContext {
    pub context: Context,
    pub trapframe: TrapFrame,
}

impl ExecutionContext {
    pub const fn empty() -> Self {
        Self {
            context: Context::zero(),
            trapframe: TrapFrame::zero(),
        }
    }
}

#[cfg(not(any(test, feature = "std")))]
extern "C" {
    pub fn context_switch(old: *mut Context, new: *const Context);
}

#[cfg(any(test, feature = "std"))]
pub unsafe fn context_switch(_old: *mut Context, _new: *const Context) {}

const _: () = {
    assert!(core::mem::size_of::<Context>() == 112);
};
