// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]

use spin::Mutex;

use x86_64::structures::idt::Idt;
use x86_64::structures::idt::{ExceptionStackFrame, PageFaultErrorCode};

use self::pic::ChainedPICs;
pub use self::keyboard::KEYBOARD;

/// Abstraction of the PS/2 keyboard
mod keyboard;
/// IO abstractions in Rust
mod cpuio;
/// The programmable interrupt controller
mod pic;

extern "C" {
    /// Enable interrupts
    fn sti();
}

lazy_static! {
    /// This is the Interrupt Descriptor Table that contains handlers for all
    /// interrupt vectors that we support. Each handler is set in its initialization
    /// and is not modified again.
    static ref IDT: Idt = {
        let mut idt = Idt::new();

        // Initialize handlers
        idt.divide_by_zero.set_handler_fn(de_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(df_handler);
        idt.general_protection_fault.set_handler_fn(gp_handler);
        idt.page_fault.set_handler_fn(pf_handler);
        // PIC handlers
        idt[0x20].set_handler_fn(timer_handler);
        idt[0x21].set_handler_fn(kb_handler);

        idt
    };
}

/// The Rust interface to the 8086 Programmable Interrupt Controller
pub static PIC: Mutex<ChainedPICs> = Mutex::new(unsafe { ChainedPICs::new(0x20, 0x28) });

pub fn init() {
    IDT.load();
    unsafe {
        {
            let mut pic = PIC.lock();
            pic.initialize();
        }
        sti();
    }
}

//  Exceptions:
//  | Name                          | Vector #   |    Type     | Mnemonic | Error Code? |
//  | ----------------------------- | ---------- | ----------- | -------- | ----------- |
//  | Divide by Zero                | 0  (0x0)   | Fault       | #DE      | No          |
//  | Debug                         | 1  (0x1)   | Both        | #DB      | No          |
//  | Non-maskable Interrupt        | 2  (0x2)   | Interrupt   | -        | No          |
//  | Breakpoint                    | 3  (0x3)   | Trap        | #BP      | No          |
//  | Overflow                      | 4  (0x4)   | Trap        | #OF      | No          |
//  | Bound Range Exceeded          | 5  (0x5)   | Fault       | #BR      | No          |
//  | Invalid Opcode                | 6  (0x6)   | Fault       | #UD      | No          |
//  | Device not Available          | 7  (0x7)   | Fault       | #NM      | No          |
//  | Double Fault                  | 8  (0x8)   | Abort       | #DF      | No          |
//  | ~Coprocessor Segment Overrun~ | 9  (0x9)   | Fault       | -        | No          |
//  | Invalid TSS                   | 10 (0xA)   | Fault       | #TS      | Yes         |
//  | Segment not Present           | 11 (0xB)   | Fault       | #NP      | Yes         |
//  | Stack-Segment Fault           | 12 (0xC)   | Fault       | #SS      | Yes         |
//  | General Protection Fault      | 13 (0xD)   | Fault       | #GP      | Yes         |
//  | Page Fault                    | 14 (0xE)   | Fault       | #PF      | Yes         |
//  | Reserved                      | 15 (0xF)   | -           | -        | No          |
//  | x87 Floating Point Exception  | 16 (0x10)  | Fault       | #MF      | No          |
//  | Alignment Check               | 17 (0x11)  | Fault       | #AC      | Yes         |
//  | Machine Check                 | 18 (0x12)  | Fault       | #MC      | No          |
//  | SIMD Floating-Point Exception | 19 (0x13)  | Fault       | #XM/#XF  | No          |
//  | Virtualisation Exception      | 20 (0x14)  | -           | #VE      | No          |
//  | Reserved                      | 21 (0x15)  | -           | -        | No          |
//  | Security Exception            | 22 (0x16)  | -           | #SX      | Yes         |
//  | Reserved                      | 23 (0x17)  | -           | -        | No          |
//  | Triple Fault                  | 24 (0x15)  | -           | -        | No          |
//  | FPU Error Interrupt           | 25 (0x18)  | Interrupt   | #FERR    | No          |
//  | ----------------------------- | ---------- | ----------- | -------- | ----------- |

/// Divide by zero handler
///
/// Occurs when the hardware attempts to divide by zero. Unrecoverable.
extern "x86-interrupt" fn de_handler(stack_frame: &mut ExceptionStackFrame) {
    panic!("EXCEPTION DIVIDE BY ZERO\n{:#?}", stack_frame);
}

/// Breakpoint handler
///
/// A harmless interrupt, operation is safely resumed after printing a message.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut ExceptionStackFrame) {
    println!("Breakpoint at {:#?}\n{:#?}",
             (stack_frame).instruction_pointer,
             stack_frame);
}

/// Double Fault handler
///
/// A double fault can occur in the following conditions:
///
/// First Exception          | Second Exception
/// ------------------------ | ------------------------
/// Divide-by-Zero           | Invalid TSS
/// Invalid TSS              | Segment Not Present
/// Segment not Present      | Stack-Segment Fault
/// Stack-Segment Fault      | General Protection Fault
/// General Protection Fault |
/// -------------------------| ------------------------
/// Page Fault               | Page Fault
///                          | Invalid TSS
///                          | Segment Not Present
///                          | Stack-Segment Fault
///                          | General Protection Fault
/// ------------------------ | ------------------------
extern "x86-interrupt" fn df_handler(stack_frame: &mut ExceptionStackFrame, _: u64) {
    println!("\nEXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame)
}

/// General Protection Fault handler
///
/// A General Protection Fault may occur for various reasons: The most common
/// are:
/// + Segment Error (privilege, type, limit, read/write rights)
/// + Executing a privileged instruction while CPL != 0
/// + Writing 1 in a reserved register field
/// + Referencing or accessing a null-descriptor
///
/// *Error Code*: The General Protection Fault error code is the segment
/// selector index when the exception is segment related, otherwise, 0.
extern "x86-interrupt" fn gp_handler(stack_frame: &mut ExceptionStackFrame, error_code: u64) {
    panic!("EXCEPTION GENERAL PROTECTION FAULT\nerror_code: {}\n{:#?}\n", error_code, stack_frame);
}

/// Page Fault handler
///
/// A Page Fault occurs when:
/// + A page directory or table entry is not present in physical memory.
/// + Attempting to load the instruction tlb with an address for a
/// non-executable page.
/// + A protection check (privileges, read/write) failed.
/// + A reserved bit in the page directory or table entries is set to 1.
extern "x86-interrupt" fn pf_handler(stack_frame: &mut ExceptionStackFrame, error_code: PageFaultErrorCode) {
    panic!("EXCEPTION PAGE FAULT\nerror_code: {:?}\n{:#?}", error_code, stack_frame);
}

/// Timer handler
extern "x86-interrupt" fn timer_handler(_: &mut ExceptionStackFrame) {
    unsafe {
        PIC.lock().master.end_of_interrupt();
    }
}

/// Keyboard handler
///
/// This function pages the `Keyboard` port to get the key that was pressed, it then
/// prints the associated byte to the screen and saves the state of the keyboard.
extern "x86-interrupt" fn kb_handler(_: &mut ExceptionStackFrame) {
    let mut kb = KEYBOARD.lock();
    match kb.port.read() {
        // If the key was just pressed,
        // then the top bit of it is unset
        x if x & 0x80 == 0 => {
            kb.keys[x as usize] = true;
            let mut byte = kb.kbmap[x as usize];

            // If either shift is pressed, make it
            // capital as long as it is alphabetic
            byte -= 0x20 * ((kb.keys[42] || kb.keys[54]) && byte > 96 && byte < 123) as u8;
            print!("{}", byte as char);
        }
        // If this runs a key was released
        // load a false into kb.keys at that point
        x => {
            let x = x & !0x80;
            kb.keys[x as usize] = false;
        }
    }
    unsafe {
        PIC.lock().master.end_of_interrupt();
    }
}
