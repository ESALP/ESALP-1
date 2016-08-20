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
	fn inb (num: u8 ) -> u8;
	fn outb(num: u8 );
	fn inw (num: u16) -> u16
	fn outw(num: u16);
	fn inl (num: u32) -> u32;
	fn outl(num: u32);
}
