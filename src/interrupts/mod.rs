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
use vga_buffer::print_error;
use self::pic::ChainedPICs;
pub use self::cpuio::Port;

mod cpuio;
mod pic;
mod idt;

extern {
	fn divide_by_zero() -> !;
	fn KEXIT() -> !;
}

lazy_static! {
	static ref IDT: idt::Idt = {
		let mut idt = idt::Idt::new();
		idt.set_handler(0,divide_by_zero);
		idt
	};
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

pub fn init() {
	IDT.load();
}

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionStackFrame {
	instruction_pointer: u64,
	code_segment: u64,
	cpu_flags: u64,
	stack_pointer: u64,
	stack_segment: u64,
}
#[derive(Debug)]
#[repr(C)]
pub struct EExceptionStackFrame {
	instruction_pointer: u64,
	code_segment: u64,
	cpu_flags: u64,
	stack_pointer: u64,
	stack_segment: u64,
	error_code: u64,
}

/*  Exceptions:
 *  | Name                          | Vector #   |    Type     | Mnemonic   | Error Code?   |
 *  | ----------------------------- | ---------- | ----------- | ---------- | ------------- |
 *  | Divide by Zero                | 0  (0x0)   | Fault       | #DE        | No            |
 *  | Debug                         | 1  (0x1)   | Both        | #DB        | No            |
 *  | Non-maskable Interrupt        | 2  (0x2)   | Interrupt   | -          | No            |
 *  | Breakpoint                    | 3  (0x3)   | Trap        | #BP        | No            |
 *  | Overflow                      | 4  (0x4)   | Trap        | #OF        | No            |
 *  | Bound Range Exceeded          | 5  (0x5)   | Fault       | #BR        | No            |
 *  | Invalid Opcode                | 6  (0x6)   | Fault       | #UD        | No            |
 *  | Device not Availible          | 7  (0x7)   | Fault       | #NM        | No            |
 *  | Double Fault                  | 5  (0x5)   | Abort       | #DF        | No            |
 *  | ~Coprocessor Segment Overrun~ | 8  (0x8)   | Fault       | -          | No            |
 *  | Invalid TSS                   | 10 (0xA)   | Fault       | #TS        | Yes           |
 *  | Segment not Present           | 11 (0xB)   | Fault       | #NP        | Yes           |
 *  | Stack-Segment Fault           | 12 (0xC)   | Fault       | #SS        | Yes           |
 *  | General Protection Fault      | 13 (0xD)   | Fault       | #GP        | Yes           |
 *  | Page Fault                    | 14 (0xE)   | Fault       | #PF        | Yes           |
 *  | Reserved                      | 15 (0xF)   | -           | -          | No            |
 *  | x87 Floating Point Exception  | 16 (0x10)  | Fault       | #MF        | No            |
 *  | Alignment Check               | 17 (0x11)  | Fault       | #AC        | Yes           |
 *  | Machine Check                 | 18 (0x12)  | Fault       | #MC        | No            |
 *  | SIMD Floating-Point Exception | 19 (0x13)  | Fault       | #XM/#XF    | No            |
 *  | Virtualization Exception      | 20 (0x14)  | -           | #VE        | No            |
 *  | Reserved                      | 21 (0x15)  | -           | -          | No            |
 *  | Security Exception            | 22 (0x16)  | -           | #SX        | Yes           |
 *  | Reserved                      | 23 (0x17)  | -           | -          | No            |
 *  | Triple Fault                  | 24 (0x15)  | -           | -          | No            |
 *  | FPU Error Interrupt           | 25 (0x18)  | Interrupt   | #FERR      | No            |
 *  | ----------------------------- | ---------- | ----------- | ---------- | ------------- |
 */

#[no_mangle]
pub extern "C" fn rust_de_interrupt_handler(stack_frame: *const ExceptionStackFrame)
	-> !
{
	unsafe {
		print_error(format_args!("EXCEPTION DIVIDE BY ZERO\n{:#?}",
								 *stack_frame));
		KEXIT();
	}
}
