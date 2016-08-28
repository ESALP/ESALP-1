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
// These are taken from the OS Dev wiki,
// not all of them are used. However they
// will most likely be useful.
const PIC1:           u16 = 0x20;    /* IO base address for master PIC */
const PIC2:           u16 = 0xA0;    /* IO base address for slave PIC */
const PIC1_COMMAND:   u16 = PIC1;
const PIC1_DATA:      u16 = (PIC1+1);
const PIC2_COMMAND:   u16 = PIC2;
const PIC2_DATA:      u16 = (PIC2+1);

const ICW1_ICW4:      u8  = 0x01;    /* ICW4 (not) needed */
const ICW1_SINGLE:    u8  = 0x02;    /* Single (cascade) mode */
const ICW1_INTERVAL4: u8  = 0x04;    /* Call address interval 4 (8) */
const ICW1_LEVEL:     u8  = 0x08;    /* Level triggered (edge) mode */
const ICW1_INIT:      u8  = 0x10;    /* Initialization - required! */

const ICW4_8086:      u8  = 0x01;    /* 8086/88 (MCS-80/85) mode */
const ICW4_AUTO:      u8  = 0x02;    /* Auto (normal) EOI */
const ICW4_BUF_SLAVE: u8  = 0x08;    /* Buffered mode/slave */
const ICW4_BUF_MASTER:u8  = 0x0C;    /* Buffered mode/master */
const ICW4_SFNM:      u8  = 0x10;    /* Special fully nested (not) */

const PIC_EOI:        u8  = 0x20;    /* End-of-Interrupt command code */

use interrupts::cpuio::{Port, UnsafePort};

pub struct PIC {
	offset: u8,
	command: UnsafePort<u8>,
	data: UnsafePort<u8>,
}
impl PIC {
	fn handles_interrupt(&self, interrupt: u8) -> bool {
		self.offset <= interrupt &&
			interrupt <= self.offset + 8
	}

	pub unsafe fn end_of_interrupt(&mut self) {
		self.command.write(PIC_EOI);
	}

	pub unsafe fn set_mask(&mut self, irq: u8) {
		assert!(irq < 8);
		let value = self.data.read() | (1 << irq);
		self.data.write(value);
	}

	pub unsafe fn clear_mask(&mut self, irq: u8) {
		assert!(irq < 8);
		let value = self.data.read() | !(1 << irq);
		self.data.write(value);
	}
}

pub struct ChainedPICs {
	pub master: PIC,
	pub slave: PIC,
}
impl ChainedPICs {
	pub const unsafe fn new(offset1: u8, offset2: u8) -> ChainedPICs {
		ChainedPICs {
			master: PIC {
				offset: offset1,
				command: UnsafePort::new(PIC1_COMMAND),
				data: UnsafePort::new(PIC1_DATA),
			},
			slave: PIC {
				offset: offset2,
				command: UnsafePort::new(PIC2_COMMAND),
				data: UnsafePort::new(PIC2_DATA),
			},
		}
	}

	pub unsafe fn initialize(&mut self) {
		// wait_io() waits for an IO opperation to complete
		// it does this by writing one number to an "unused"
		// port, which takes the amount of time that we want.
		// It writes to 0x80 which is used for 'checkpoints'
		// during POST, but Linux considers it to be unused
		// and we will too.
		let mut wait_port: Port<u8> = Port::new(0x80);
		let mut wait = || wait_port.write(0);

		//save masks
		let masks = (self.master.data.read(), self.slave.data.read());

		// begin 3 byte initialization sequence.
		self.master.command.write(ICW1_INIT | ICW1_ICW4);
		wait();
		self.slave.command.write(ICW1_INIT | ICW1_ICW4);
		wait();

		// send byte 1: offset
		self.master.data.write(self.master.offset);
		wait();
		self.slave.data.write(self.slave.offset);
		wait();

		// send byte 2: wiring
		self.master.data.write(4); //slave at irq2
		wait();
		self.slave.data.write(2); //identity of slave
		wait();

		// send byte 3: additional environment info
		self.master.data.write(ICW4_8086); //8086 mode
		wait();
		self.slave.data.write(ICW4_8086); //8086 mode
		wait();

		// restore masks
		self.master.data.write(masks.0);
		self.slave.data.write(masks.1);
	}

	pub fn handles_interrupt(&self, interrupt: u8) -> bool {
		self.master.handles_interrupt(interrupt) &&
			self.slave.handles_interrupt(interrupt)
	}

	pub unsafe fn set_mask(&mut self, irq: u8) {
		assert!(irq < 16);
		if irq < 8 {
			self.master.set_mask(irq)
		}
		else {
			self.slave.set_mask(irq - 8)
		}
	}

	pub unsafe fn clear_mask(&mut self, irq: u8) {
		assert!(irq < 16);
		if irq < 8 {
			self.master.clear_mask(irq)
		}
		else {
			self.slave.clear_mask(irq - 8)
		}
	}
}
