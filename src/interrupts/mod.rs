/*
 * Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
 * file at the top-level directory of this distribution.
 *
 * Licensed under the MIT license <LICENSE or
 * http://opensource.org/licenses/MIT>, at your option.
 * This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use spin::Mutex;
pub use self::port::Port;

mod port;

pub static KEYBOARD: Mutex<Port<u8>> = Mutex::new(
	unsafe {
		Port::new(0x60)
	}
);
