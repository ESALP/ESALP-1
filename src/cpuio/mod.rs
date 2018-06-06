// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(unused_macros)]

use spin::Mutex;
use self::serial::Serial;

pub static COM1: Mutex<Serial> = Mutex::new( unsafe {
    Serial::new(serial::COM1)
});

macro_rules! serial_println {
    ($fmt:expr) => (serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (serial_print!(concat!($fmt, "\n"), $($arg)*));
}

macro_rules! serial_print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        if let Some(mut serial) = $crate::cpuio::COM1.try_lock() {
            serial.write_fmt(format_args!($($arg)*)).unwrap();
        }
    });
}


pub mod port;
pub mod serial;

pub fn init() {
    COM1.lock().init();
}

#[cfg(feature = "test")]
pub mod tests {
    pub fn run() {
        super::serial::tests::run();
    }
}
