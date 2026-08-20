#[cfg(target_os = "none")]
use crate::config::UART_BASE;

#[cfg(target_os = "none")]
pub fn putc(c: u8) {
    unsafe {
        let lsr = (UART_BASE + 5) as *const u8;
        let thr = UART_BASE as *mut u8;
        for _ in 0..10_000 {
            if core::ptr::read_volatile(lsr) & (1 << 5) != 0 {
                break;
            }
        }
        core::ptr::write_volatile(thr, c);
    }
}

#[cfg(not(target_os = "none"))]
pub fn putc(c: u8) {
    let _ = c;
}

pub fn write_str(s: &str) {
    for b in s.bytes() {
        if b == b'\n' {
            putc(b'\r');
        }
        putc(b);
    }
}
