use crate::uart;
use core::fmt::{self, Write};

pub fn write_str(s: &str) {
    uart::write_str(s);
}

pub fn write_usize(n: usize) {
    if n == 0 {
        uart::putc(b'0');
        return;
    }
    let mut digits = [0u8; 20];
    let mut i = 0;
    let mut num = n;
    while num > 0 && i < 20 {
        digits[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        uart::putc(digits[i]);
    }
}

struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        uart::write_str(s);
        Ok(())
    }
}

pub fn print_fmt(args: fmt::Arguments) {
    let _ = Writer.write_fmt(args);
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::console::print_fmt(core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! kprintln {
    () => { $crate::kprint!("\n") };
    ($($arg:tt)*) => { $crate::kprint!("{}\n", core::format_args!($($arg)*)) };
}
