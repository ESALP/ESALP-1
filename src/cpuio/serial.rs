// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]

use super::port::Port;

pub const COM1: u16 = 0x3F8;
pub const COM2: u16 = 0x2F8;
pub const COM3: u16 = 0x3E8;
pub const COM4: u16 = 0x2e8;

bitflags! {
    flags LineControl: u8 {
        const LENB1 = 1 << 0,
        const LENB2 = 1 << 1,
        const LEN5 = 0,
        const LEN6 = LENB1.bits,
        const LEN7 = LENB2.bits,
        const LEN8 = LENB1.bits | LENB2.bits,

        const STOP1 = 0 << 2,
        const STOP2 = 1 << 2,

        const PARITYB1 = 1 << 3,
        const PARITYB2 = 1 << 4,
        const PARITYB3 = 1 << 5,
        const PARITY_NONE = 0,
        const PARITY_ODD = PARITYB1.bits,
        const PARITY_EVEN = PARITYB1.bits | PARITYB2.bits,
        const PARITY_MARK = PARITYB1.bits | PARITYB3.bits,
        const PARITY_SPACE = PARITYB1.bits | PARITYB2.bits |
            PARITYB3.bits,

        const DLAB_ENABLE = 1 << 7,
    }
}
bitflags! {
    flags InterruptEnable: u8 {
        const DATA_AVAILABLE = 1 << 0,
        const TRANSMITTER_EMPTY = 1 << 1,
        const BREAK_ERROR = 1 << 2,
        const STATUS_CHANGE = 1 << 3,
    }
}

pub struct Serial {
    data: Port<u8>,
    interrupt_enable: Port<u8>,
    fifo: Port<u8>,
    line_ctrl: Port<u8>,
    modem_ctrl: Port<u8>,
    line_stat: Port<u8>,
    modem_stat: Port<u8>,
    scratch: Port<u8>,
}
impl Serial {
    /// Create a new Serial port with the given base
    pub const unsafe fn new(base: u16) -> Self {
        Serial {
            data: Port::new(base + 0),
            interrupt_enable: Port::new(base + 1),
            fifo: Port::new(base + 2),
            line_ctrl: Port::new(base + 3),
            modem_ctrl: Port::new(base + 4),
            line_stat: Port::new(base + 5),
            modem_stat: Port::new(base + 6),
            scratch: Port::new(base + 7),
        }
    }
    /// Initialize the port with 8 bits, no parity, 1 stop bit, and a divisor of
    /// 3. Why? It's how the osdev wiki does it.
    pub fn init(&mut self) {
        self.interrupt_enable.write(0); // disable all interrupts
        self.line_ctrl.write(DLAB_ENABLE.bits); // Enable DLAB
        self.data.write(0x03); // Set divisor of 3, 38400 baud
        self.interrupt_enable.write(0x00); // high bit
        self.line_ctrl.write((LEN8 | STOP1 | PARITY_NONE).bits);
        self.fifo.write(0xC7);
        self.modem_ctrl.write(0xB);

    }

    fn serial_recieved(&mut self) -> bool {
        self.line_stat.read() & 1 == 0
    }

    pub fn read(&mut self) -> u8 {
        while self.serial_recieved() {}

        self.data.read()
    }

    fn is_transmit_empty(&mut self) -> bool {
        self.line_stat.read() & 0x20 != 0
    }

    pub fn write(&mut self, a: u8) {
        while !self.is_transmit_empty() {}

        self.data.write(a);
    }
}

impl ::core::fmt::Write for Serial {
    /// Writes a `&str` to the serial bus
    ///
    /// # Errors
    /// Always returns Ok(())
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        for byte in s.bytes() {
            self.write(byte);
        }
        Ok(())
    }
}
