/*
 * Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
 * file at the top-level directory of this distribution.
 *
 * Licensed under the MIT license <LICENSE or
 * http://opensource.org/licenses/MIT>, at your option.
 * This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use x86::segmentation::{self,SegmentSelector};
use bit_field::BitField;

pub type HandlerFunc = unsafe extern "C" fn();

pub struct Idt([Entry; 40]);

impl Idt {
	pub fn new() -> Self {
		Idt([Entry::missing(); 40])
	}

	pub fn set_handler(&mut self, entry: u8, handler: HandlerFunc)
		-> &mut EntryOptions
	{
		self.0[entry as usize] = Entry::new(segmentation::cs(),handler);
		&mut self.0[entry as usize].options
	}

	// In order to insure safety, self must live
	// for the entirety of the kernel. A lifetime
	// of 'static ensures this
	pub fn load(&'static self) {
		use x86::dtables::{DescriptorTablePointer, lidt};
		use core::mem::size_of;

		let ptr = DescriptorTablePointer {
			base: self as *const _ as u64,
			limit: (size_of::<Self>() - 1) as u16,
		};
		unsafe{ lidt(&ptr) }
	}
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Entry {
	pointer_low: u16,
	gdt_selector: SegmentSelector,
	options: EntryOptions,
	pointer_middle: u16,
	pointer_high: u32,
	reserved: u32,
}

impl Entry {
	fn new(gdt_selector: SegmentSelector, handler: HandlerFunc) -> Self {
		let pointer = handler as usize;
		Entry {
			gdt_selector: gdt_selector,
			pointer_low: pointer as u16,
			pointer_middle: (pointer >> 16) as u16,
			pointer_high: (pointer >> 32) as u32,
			options: EntryOptions::new(),
			reserved: 0,
		}
	}

	fn missing() -> Self {
		Entry {
			gdt_selector: SegmentSelector::new(0),
			pointer_low: 0,
			pointer_middle: 0,
			pointer_high: 0,
			options: EntryOptions::minimal(),
			reserved: 0,
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct EntryOptions(BitField<u16>);

impl EntryOptions {
	fn minimal() -> Self {
		let mut options = BitField::new(0);
		options.set_range(9..12, 0b111); // 'must be one' bits
		EntryOptions(options)
	}

	fn new() -> Self {
		let mut options = Self::minimal();
		options.set_present(true);
		options.disable_interrupts(true);
		options
	}

	pub fn set_present(&mut self, present: bool) {
		self.0.set_bit(15,present);
	}

	pub fn disable_interrupts(&mut self, disable: bool) {
		self.0.set_bit(8, !disable);
	}

	pub fn set_privilege_level(&mut self, dpl: u16) {
		self.0.set_range(13..15, dpl);
	}

	pub fn set_stack_index(&mut self, index: u16) {
		self.0.set_range(0..3, index);
	}
}
