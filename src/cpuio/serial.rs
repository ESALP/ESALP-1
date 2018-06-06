// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]

//! Communication using the serial port!

use super::port::Port;

pub const COM1: u16 = 0x3F8;
pub const COM2: u16 = 0x2F8;
pub const COM3: u16 = 0x3E8;
pub const COM4: u16 = 0x2E8;

bitflags! {
    /// Set transmission protocol with this register
    ///
    /// | Bit 1 | Bit 0 | word length |
    /// | ----- | ----- | ----------- |
    /// |     0 |     0 |      5 bits |
    /// |     0 |     1 |      6 bits |
    /// |     1 |     0 |      7 bits |
    /// |     1 |     1 |      8 bits |
    ///
    /// | Bit 2 | stop bits |
    /// | ----- | --------- |
    /// |     0 |         1 |
    /// |     1 |     1.5/2 |
    ///
    /// | Bit 5 | Bit 4 | Bit 3 |  parity type |
    /// | ----- | ----- | ----- | ------------ |
    /// |     x |     x |     0 |    no parity |
    /// |     0 |     1 |     1 |   odd parity |
    /// |     1 |     0 |     1 |  even parity |
    /// |     1 |     1 |     1 |  mark parity |
    /// |     1 |     1 |     1 | space parity |
    struct LineControl: u8 {
        const LENB1 = 1 << 0;
        const LENB2 = 1 << 1;
        const LEN5 = 0;

        const STOP = 1 << 2;

        /// 8n1, 8bits, no parity, 1 stop bit
        const RATE = Self::LENB1.bits | Self::LENB2.bits;

        const PARITYB1 = 1 << 3;
        const PARITYB2 = 1 << 4;
        const PARITYB3 = 1 << 5;

        const DLAB_ENABLE = 1 << 7;
    }
} bitflags! {
    /// Enable interrupts with these bits
    struct InterruptEnable: u8 {
        /// Interrupt generated when data waits to be read by the CPU
        const DATA_AVAILABLE = 1 << 0;
        /// Interrupt that tells the CPU when to write characters to the THR
        const TRANSMITTER_EMPTY = 1 << 1;
        /// Interrupt that informs the CPU of transmission errors
        const BREAK_ERROR = 1 << 2;
        /// Interrupt triggered when one of the delta-bits is set
        const STATUS_CHANGE = 1 << 3;
    }
} bitflags! {
    /// This register allows for error detection and polled-mode operation
    struct LineStatus: u8 {
        /// Error somewhere in RX FIFO chain
        const FIFOERR = 1 << 7;
        /// Transmitter is empty (last data has been sent)
        const TEMT = 1 << 6;
        /// THR empty (new data can be written)
        const THRE = 1 << 5;
        /// Broken line detected
        const BREAK = 1 << 4;
        /// Framing error
        const FE = 1 << 3;
        /// Parity error
        const PE = 1 << 2;
        /// Overrun error
        const OE = 1 << 1;
        /// Reciever buffer full (data available)
        const RBF = 1 << 0;
    }
} bitflags! {
    /// This register allows control of the FIFOs of 16550+ UART controllers
    ///
    /// Bits 6-7 change the reciever FIFO trigger level
    /// | Bit 7 | Bit 6 |  level |
    /// | ----- | ----- | ------ |
    /// |     0 |     0 |      1 |
    /// |     0 |     1 |      4 |
    /// |     1 |     0 |      8 |
    /// |     1 |     1 |     14 |
    struct FifoControl: u8 {
        /// FIFO enable. If unset all other bits are ignored
        const FE = 1 << 0;
        /// Clear receiver FIFO. This bit is self-clearing
        const RFR = 1 << 0;
        /// Clear transmitter FIFO. This bit is self-clearing
        const XFR = 1 << 0;
        /// DMA mode (probably not available and silly)
        const DMAS = 1 << 0;

        const RX1 = 1 << 6;
        const RX2 = 1 << 7;

        /// Default is `FE | RFR | XFR | RX1 | RX2`
        const DEFAULT = 0xC7;
    }
} bitflags! {
    /// This register allows programming modem control lines and loopback
    struct ModemControl: u8 {
        /// Programs -DTR (a handshaking line)
        const DTR = 1 << 0;
        /// Programs -RTS (ditto)
        const RTS = 1 << 1;
        /// Programs -OUT1
        ///
        /// Normally not used in a PC, but it's best to write 1 here.
        const OUT1 = 1 << 2;
        /// Programs -OUT2. If set to 1, interrupts are transfered to the ICU
        /// (Interrupt Control Unit)
        const OUT2 = 1 << 3;
        /// Local loopback. All outputs are disabled.
        ///
        /// This is a means of testing the chip: you 'recieve' all the data
        /// you send.
        const LOOP = 1 << 4;

        /// Default is `DTR | RTS | OUT1 | OUT2`
        const DEFAULT = 0x0F;
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
        self.line_ctrl.write(LineControl::DLAB_ENABLE.bits); // Enable DLAB
        self.data.write(0x03); // Set divisor of 3, 38400 baud
        self.interrupt_enable.write(0x00); // high bit
        self.line_ctrl.write(LineControl::RATE.bits);
        self.fifo.write(FifoControl::DEFAULT.bits);
        self.modem_ctrl.write(ModemControl::DEFAULT.bits);
    }

    fn line_status(&mut self) -> LineStatus {
        LineStatus::from_bits_truncate(self.line_stat.read())
    }

    fn serial_recieved(&mut self) -> bool {
        self.line_status().contains(LineStatus::RBF)
    }

    pub fn read(&mut self) -> u8 {
        while !self.serial_recieved() {}

        self.data.read()
    }

    fn is_transmit_empty(&mut self) -> bool {
        self.line_status().contains(LineStatus::THRE)
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

#[cfg(feature = "test")]
pub mod tests {
    use tap::TestGroup;
    use super::{Serial, ModemControl};

    pub fn run() {
        test_transmission();
    }

    // Tests COM2 because COM1 is used for tests
    fn test_transmission() {
        let mut serial = unsafe { Serial::new(super::COM2) };

        // Set COM2 to loopback mode
        serial.modem_ctrl.write(ModemControl::LOOP.bits);

        let mut tap = TestGroup::new(10);
        tap.diagnostic("Testing the UART serial port");

        for i in 20u8..30 {
            serial.write(i);
            tap.assert_tap(i == serial.read(),
                "Data written to the serial port was different from data read!");
        }
    }
}
