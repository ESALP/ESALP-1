// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use spin::Mutex;
use interrupts::cpuio::Port;

pub struct Keyboard {
    pub port: Port<u8>,
    pub kbmap: [u8; 128],
    pub keys: [bool; 128],
}
impl Keyboard {
    pub const fn new() -> Keyboard {
        Keyboard {
            port: unsafe {Port::new(0x60)},
            kbmap: KBDUS,
            keys: [false; 128]
        }
    }

    pub fn change_kbmap(&mut self, kbmap: &[u8; 128]) {
        self.kbmap = *kbmap;
    }
}

pub static KEYBOARD: Mutex<Keyboard> = Mutex::new(
    Keyboard::new()
);

const KBDUS: [u8; 128] =
[
    b'\0',  b'\x27', b'1', b'2', b'3', b'4', b'5',
    b'6', b'7', b'8',    b'9', b'0', b'-', b'=', b'\x08',/* Backspace */
    b'\t',/* Tab */
    b'q', b'w', b'e', b'r', b't', b'y',
    b'u', b'i', b'o', b'p', b'[', b']', b'\n',    /* Enter key */
     b'\0', /* 29   - Control */
    b'a', b's', b'd', b'f', b'g', b'h',
    b'j', b'k', b'l', b';', /* 39 */
    b'\'', b'`',   b'\0', /* Left shift */
    b'\\', b'z', b'x', b'c', b'v', b'b', b'n', /* 49 */
    b'm', b',', b'.', b'/',   b'\0',    /* Right shift */
    b'*',
    b'\0',    /* Alt */
    b' ',    /* Space bar */
    b'\0',    /* Caps lock */
    b'\0',    /* 59 - F1 key ... > */
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0',
    b'\0',    /* < ... F10 */
    b'\0',    /* 69 - Num lock*/
    b'\0',    /* Scroll Lock */
    b'\0',    /* Home key */
    b'\0',    /* Up Arrow */
    b'\0',    /* Page Up */
    b'-',
    b'\0',    /* Left Arrow */
    b'\0',
    b'\0',    /* Right Arrow */
    b'+',
    b'\0',    /* 79 - End key*/
    b'\0',    /* Down Arrow */
    b'\0',    /* Page Down */
    b'\0',    /* Insert Key */
    b'\0',    /* Delete Key */
    b'\0',   b'\0',   b'\0',
    b'\0',    /* F11 Key */
    b'\0',    /* F12 Key */
    b'\0',    /* All other keys are undefined */
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0',
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0',
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0',
    b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0', b'\0'
];
