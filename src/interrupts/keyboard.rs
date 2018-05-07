// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use spin::Mutex;
use cpuio::port::Port;

/// A struct that represents an interface to the PS/2 keyboard
pub struct Keyboard {
    /// The keyboard port, has to be 0x60
    pub port: Port<u8>,
    /// The keyboard mapping in ascii. Non-used characters are NUL
    pub kbmap: [u8; 128],
    /// Keyboard key state. True if pressed, false if unpressed
    pub keys: [bool; 128],
}
impl Keyboard {
    /// Returns a new `Keyboard` with the `KBDUS` layout
    pub const fn new() -> Keyboard {
        Keyboard {
            port: unsafe { Port::new(0x60) },
            kbmap: KBDUS,
            keys: [false; 128],
        }
    }

    /// This function takes a reference to a keyboard mapping and copies it into
    /// the struct.
    pub fn change_kbmap(&mut self, kbmap: &[u8; 128]) {
        self.kbmap = *kbmap;
    }
}

/// `KEYBOARD` is the default `Keyboard`
pub static KEYBOARD: Mutex<Keyboard> = Mutex::new(Keyboard::new());

/// This is the standard US keyboard layout.
const KBDUS: [u8; 128] =
    [b'\0', b'\x27', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=',
     b'\x08', b'\t', b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']',
     b'\n', b'\0', b'a', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', b'\0',
     b'\\', b'z', b'x', b'c', b'v', b'b', b'n', b'm', b',', b'.', b'/', b'\0', b'*', b'\0', b' ',
     b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0',
     b'\0', b'\0', b'\0', b'-', b'\0', b'\0', b'\0', b'+', b'\0', b'\0', b'\0', b'\0', b'\0',
     b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0',
     b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0',
     b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0',
     b'\0', b'\0', b'\0', b'\0', b'\0'];
