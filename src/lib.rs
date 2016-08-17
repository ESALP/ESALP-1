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
pub extern fn rust_main() {}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
	unsafe{ KEXIT() }
}

#[lang = "eh_personality"] extern fn eh_personality() {}
#[lang = "panic_fmt"] extern fn panic_fmt() -> ! {unsafe {KEXIT()}}
