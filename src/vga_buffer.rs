// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]

use core::ptr::Unique;
use spin::Mutex;

/// The number of rows in the VGA text buffer
const BUFFER_HEIGHT: usize = 25;
/// The number of columns in the VGA text buffer
const BUFFER_WIDTH: usize = 80;

/// All writing to the VGA text buffer _must_ go through this
/// struct.
pub static WRITER: Mutex<Writer> = Mutex::new(Writer {
    column_position: 0,
    color_code: ColorCode::new(Color::Pink, Color::Black),
    buffer: unsafe { Unique::new_unchecked(0xb8000 as *mut _) },
});

macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        if let Some(mut writer) = $crate::vga_buffer::WRITER.try_lock() {
            writer.write_fmt(format_args!($($arg)*)).unwrap();
        }
    });
}

/// Clears the VGA text buffer
pub fn clear_screen() {
    // The screen clear has to be done in one print for it to clear the screen with
    // complete gurentee.
    unsafe {
        println!("{}",::core::str::from_utf8_unchecked(&[b'\n'; BUFFER_HEIGHT]));
    }
}

/// Changes the color of the `WRITER` struct. This may produce
/// unpredictable behaviour if `bg` has the bright bit (bit 3)
/// set.
pub fn change_color(fg: Color, bg: Color) {
    WRITER.lock().color(fg, bg);
}

/// An enum to represent the color of text. Each color is only
/// four bytes, having a `Color` enum with any number greater
/// than 0xf is undefined behaviour
#[allow(dead_code)]
#[repr(u8)]
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    DarkGray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xa,
    LightCyan = 0xb,
    LightRed = 0xc,
    Pink = 0xd,
    Yellow = 0xe,
    White = 0xf,
}

/// A struct that abstracts writing to the VGA text buffer.
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: Unique<Buffer>,
}

impl Writer {
    /// Writes one ascii byte to the screen at the current position, and then advances
    /// the position.
    ///
    /// ##Some bytes produce different behaviour, these are:
    /// * `b'\0'`: Printing a NUL byte is a no-op.
    /// * `b'\t'`: Four spaces are printed.
    /// * `b'\b'`: Clears the current byte and goes back one position. _Does not move to previous
    ///   lines_.
    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\0' => (),
            b'\n' => self.new_line(),
            b'\t' => self.write_str("    "),
            b'\x08' => self.back_space(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                self.buffer().chars[row][col] = ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                };
                self.column_position += 1;
            }
        }
    }

    /// Writes a `&str` to the screen
    fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    /// Changes the color of the buffer
    fn color(&mut self, fg: Color, bg: Color) {
        self.color_code = ColorCode::new(fg, bg);
    }

    /// Gets a reference to the buffer
    fn buffer(&mut self) -> &mut Buffer {
        unsafe { self.buffer.as_mut() }
    }

    /// Moves the position to the next line, similar to printing
    /// `b"\n\r"` in a terminal
    fn new_line(&mut self) {
        for row in 0..(BUFFER_HEIGHT - 1) {
            let buffer = self.buffer();
            buffer.chars[row] = buffer.chars[row + 1]
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    /// Moves the position back one place and clears the current byte
    fn back_space(&mut self) {
        let row = BUFFER_HEIGHT - 1;
        let col = self.column_position;
        if col == 0 {
            return;
        } else {
            self.buffer().chars[row][col - 1] = ScreenChar {
                ascii_character: b' ',
                color_code: self.color_code,
            };
            self.column_position = col - 1;
        }
    }

    /// Clears the entire current row
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        self.buffer().chars[row] = [blank; BUFFER_WIDTH];
    }
}

impl ::core::fmt::Write for Writer {
    /// Writes a `&str` to the screen
    ///
    /// # Errors
    /// Always returns Ok(())
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// A color code containing a foreground and background `Color`
#[derive(Clone, Copy)]
struct ColorCode(u8);

impl ColorCode {
    /// Combines `fg` and `bg` to form a ColorCode
    const fn new(fg: Color, bg: Color) -> ColorCode {
        ColorCode((bg as u8) << 4 | (fg as u8))
    }
}

/// A representation of one character on the VGA text buffer consisting
/// of both a ascii byte and a `ColorCode`
#[repr(C)]
#[derive(Clone, Copy)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// A representation of the entire VGA text buffer in memory
///
/// # Safety
/// The only VGA text buffer used by the kernel is the one
/// at `0xb8000`, therefore this struct is only valid there.
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}
