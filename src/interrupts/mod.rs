/*
 * Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
 * file at the top-level directory of this distribution.
 *
 * Licensed under the MIT license <LICENSE or
 * http://opensource.org/licenses/MIT>, at your option.
 * This file may not be copied, modified, or distributed
 * except according to those terms.
 */

#![allow(dead_code)]

use spin::Mutex;
use self::pic::ChainedPICs;
pub use self::cpuio::Port;

mod cpuio;
mod pic;
mod idt;

lazy_static! {
	static ref IDT: idt::Idt = idt::Idt::new();
}

pub static KEYBOARD: Mutex<Port<u8>> = Mutex::new(
	unsafe {
		Port::new(0x60)
	}
);
pub static PIC: Mutex<ChainedPICs> = Mutex::new(
	unsafe {
		ChainedPICs::new(0x20,0x28)
	}
);
