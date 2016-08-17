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
#![no_std]

extern crate rlibc;

extern {
	fn KEXIT() -> !;
}

#[no_mangle]
pub extern fn rust_main() {
	let mut message = [0x3f;34];
	{
		let hello = b"\x02\x01 Hello Rust! \x01\x02";

		for (i,char_byte) in hello.into_iter().enumerate() {
			message[i * 2] = *char_byte;
		}
	}
	// write msg to the center of the text buffer
	let buffer_ptr = (0xb8000 + 1988) as *mut _;
	unsafe { *buffer_ptr = message };

	unsafe{ KEXIT() }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
	unsafe{ KEXIT() }
}

#[lang = "eh_personality"] extern fn eh_personality() {}
#[lang = "panic_fmt"] extern fn panic_fmt() -> ! {unsafe {KEXIT()}}
