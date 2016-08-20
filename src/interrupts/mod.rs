/*
 * Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
 * file at the top-level directory of this distribution.
 *
 * Licensed under the MIT license <LICENSE or
 * http://opensource.org/licenses/MIT>, at your option.
 * This file may not be copied, modified, or distributed
 * except according to those terms.
 */
extern{
	fn inb (port: u16) -> u8;
	fn outb(port: u16, value: u8);
	fn inw (port: u16) -> u16
	fn outw(port: u16, value: u16);
	fn inl (port: u16) -> u32;
	fn outl(port: u16, value: u32);
}
