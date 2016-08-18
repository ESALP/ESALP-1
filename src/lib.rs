/*
 * Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
 * file at the top-level directory of this distribution.
 * 
 * Licensed under the MIT license <LICENSE or
 * http://opensource.org/licenses/MIT>, at your option.
 * This file may not be copied, modified, or distributed
 * except according to those terms.
 */
#![feature(lang_items)]
#![feature(const_fn, unique)]
#![no_std]

extern crate rlibc;
extern crate spin;

#[macro_use]
mod vga_buffer;

extern {
	fn KEXIT() -> !;
}

#[no_mangle]
pub extern fn rust_main() {
	use core::fmt::Write;
	vga_buffer::clear_screen();
	println!("Hello Rust log");
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern fn _Unwind_Resume() -> ! {
	unsafe{ KEXIT() }
}

#[lang = "eh_personality"] extern fn eh_personality() {}
#[lang = "panic_fmt"]
extern fn panic_fmt(args: ::core::fmt::Arguments,
					file: &'static str,
					line: u32) -> ! {
	vga_buffer::WRITER.lock().color(vga_buffer::Color::Red,
									vga_buffer::Color::Black);
	println!("\n\nPANIC at {}:{}", file, line);
	println!("\t{}",args);
	unsafe { KEXIT() }
}
