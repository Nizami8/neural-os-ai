use crate::trapframe::TrapFrame;

const MCAUSE_INT: usize = 1 << 63;
const CAUSE_MTIMER: usize = 7;
const CAUSE_ECALL_U: usize = 8;
const CAUSE_ECALL_M: usize = 11;
const CAUSE_ECALL_S: usize = 9;

#[no_mangle]
pub extern "C" fn rust_trap_handler(tf: *mut TrapFrame) -> *mut TrapFrame {
    unsafe {
        #[allow(static_mut_refs)]
        let k = &mut crate::kernel::KERNEL;
        let tf = &mut *tf;
        let mcause = tf.mcause;

        if mcause & MCAUSE_INT != 0 {
            if mcause & 0xfff == CAUSE_MTIMER {
                k.on_timer();
                crate::timer::ack();
                if k.want_resched {
                    k.reschedule();
                }
            }
        } else {
            match mcause & 0xfff {
                CAUSE_ECALL_U | CAUSE_ECALL_S | CAUSE_ECALL_M => {
                    crate::syscall::dispatch(k, tf);
                    if k.want_resched {
                        k.reschedule();
                    }
                }
                cause => {
                    crate::console::write_str("unexpected trap cause=");
                    crate::console::write_usize(cause);
                    crate::console::write_str(" epc=");
                    crate::console::write_usize(tf.mepc);
                    crate::console::write_str("\n");
                    k.sys_exit();
                    k.want_resched = true;
                    k.reschedule();
                }
            }
        }

        k.current_trapframe_ptr()
    }
}

#[cfg(target_os = "none")]
extern "C" {
    pub fn trap_return(tf: *mut TrapFrame) -> !;
}

#[cfg(not(target_os = "none"))]
pub unsafe fn trap_return(_tf: *mut TrapFrame) -> ! {
    loop {}
}
