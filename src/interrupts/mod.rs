// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code)]

use core::fmt;

use spin::Mutex;

use self::pic::ChainedPICs;
pub use self::keyboard::KEYBOARD;
use vga_buffer;

mod keyboard;
mod cpuio;
mod pic;
mod idt;

extern "C" {
    fn isr0();
    fn isr3();
    fn isr13();
    fn isr14();
    fn isr32();
    fn isr33();
    fn sti();
    fn KEXIT();
}

lazy_static! {
    static ref IDT: idt::Idt = {
        let mut idt = idt::Idt::new();

        // Initialize handlers
        idt.set_handler(0x0, isr0 );
        idt.set_handler(0x3, isr3 );
        idt.set_handler(0xD, isr13);
        idt.set_handler(0xE, isr14);
        idt.set_handler(0x20,isr32);
        idt.set_handler(0x21,isr33);
        idt
    };
}

pub static PIC: Mutex<ChainedPICs> = Mutex::new(unsafe { ChainedPICs::new(0x20, 0x28) });

pub fn init() {
    IDT.load();
    unsafe {
        {
            let mut pic = PIC.lock();
//            pic.set_mask(0);
            pic.initialize();
        }
        sti();
    }
}

#[repr(C)]
pub struct ExceptionStackFrame {
    error_code: u64,
    instruction_pointer: u64,
    code_segment: u64,
    cpu_flags: u64,
    stack_pointer: u64,
    stack_segment: u64,
}

impl fmt::Debug for ExceptionStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, r#"
ExceptionStackFrame {{
    Instruction Pointer: 0x{:04x}:{:0al$x},
    Stack Pointer:       0x{:04x}:{:0al$x},
    Flags:               0b{:0fl$b},
    Error Code:          0b{:0fl$b},
}}"#,   self.code_segment, self.instruction_pointer,
        self.stack_segment,self.stack_pointer,
        self.cpu_flags, self.error_code,
        // TODO maybe adjust this dynamically?
        al = 16,
        fl = 16)
    }
}

//  Exceptions:
//  | Name                          | Vector #   |    Type     | Mnemonic   | Error Code?   |
//  | ----------------------------- | ---------- | ----------- | ---------- | ------------- |
//  | Divide by Zero                | 0  (0x0)   | Fault       | #DE        | No            |
//  | Debug                         | 1  (0x1)   | Both        | #DB        | No            |
//  | Non-maskable Interrupt        | 2  (0x2)   | Interrupt   | -          | No            |
//  | Breakpoint                    | 3  (0x3)   | Trap        | #BP        | No            |
//  | Overflow                      | 4  (0x4)   | Trap        | #OF        | No            |
//  | Bound Range Exceeded          | 5  (0x5)   | Fault       | #BR        | No            |
//  | Invalid Opcode                | 6  (0x6)   | Fault       | #UD        | No            |
//  | Device not Availible          | 7  (0x7)   | Fault       | #NM        | No            |
//  | Double Fault                  | 8  (0x8)   | Abort       | #DF        | No            |
//  | ~Coprocessor Segment Overrun~ | 9  (0x9)   | Fault       | -          | No            |
//  | Invalid TSS                   | 10 (0xA)   | Fault       | #TS        | Yes           |
//  | Segment not Present           | 11 (0xB)   | Fault       | #NP        | Yes           |
//  | Stack-Segment Fault           | 12 (0xC)   | Fault       | #SS        | Yes           |
//  | General Protection Fault      | 13 (0xD)   | Fault       | #GP        | Yes           |
//  | Page Fault                    | 14 (0xE)   | Fault       | #PF        | Yes           |
//  | Reserved                      | 15 (0xF)   | -           | -          | No            |
//  | x87 Floating Point Exception  | 16 (0x10)  | Fault       | #MF        | No            |
//  | Alignment Check               | 17 (0x11)  | Fault       | #AC        | Yes           |
//  | Machine Check                 | 18 (0x12)  | Fault       | #MC        | No            |
//  | SIMD Floating-Point Exception | 19 (0x13)  | Fault       | #XM/#XF    | No            |
//  | Virtualization Exception      | 20 (0x14)  | -           | #VE        | No            |
//  | Reserved                      | 21 (0x15)  | -           | -          | No            |
//  | Security Exception            | 22 (0x16)  | -           | #SX        | Yes           |
//  | Reserved                      | 23 (0x17)  | -           | -          | No            |
//  | Triple Fault                  | 24 (0x15)  | -           | -          | No            |
//  | FPU Error Interrupt           | 25 (0x18)  | Interrupt   | #FERR      | No            |
//  | ----------------------------- | ---------- | ----------- | ---------- | ------------- |

#[no_mangle]
pub extern "C" fn rust_irq_handler(stack_frame: *const ExceptionStackFrame,
                                   isr_number: usize) {
	match isr_number {
        0x0 => rust_de_handler(stack_frame),
        0x3 => breakpoint_handler(stack_frame),
        0xD => rust_gp_handler(stack_frame),
        0xE => rust_pf_handler(stack_frame),
        0x20 => rust_timer_handler(),
        0x21 => rust_kb_handler(),
        _   => unreachable!(),
    }
}

extern "C" fn rust_de_handler(stack_frame: *const ExceptionStackFrame) {
    unsafe {
        panic!("EXCEPTION DIVIDE BY ZERO\n{:#?}", *stack_frame);
    }
}

extern "C" fn breakpoint_handler(stack_frame: *const ExceptionStackFrame) {
    unsafe {
        println!("Breakpoint at {:#?}\n{:#?}",
                                   (*stack_frame).instruction_pointer,
                                   *stack_frame);
    }
}

extern "C" fn rust_gp_handler(stack_frame: *const ExceptionStackFrame) {
    unsafe {
        panic!("EXCEPTION GENERAL PROTECTION FAULT\n{:#?}", *stack_frame);
    }
}

extern "C" fn rust_pf_handler(stack_frame: *const ExceptionStackFrame) {
    unsafe {
        panic!("EXCEPTION PAGE FAULT\n{:#?}", *stack_frame);
    }
}

extern "C" fn rust_timer_handler() {
	// print to the screen
	vga_buffer::flush_screen();
	unsafe {
		PIC.lock().master.end_of_interrupt();
	}
}

extern "C" fn rust_kb_handler() {
    let mut kb = KEYBOARD.lock();
//    panic!("KEYBOARD");
    match kb.port.read() {
        // If the key was just pressed,
        // then the top bit of it is set
        x if x & 0x80 == 0 => {
            kb.keys[x as usize] = true;
            let mut byte: u8 = kb.kbmap[x as usize];

            // If either shift is pressed, make it
            // capital as long as it is alphabetic
            byte -= 0x20 *
                    ((kb.keys[42] || kb.keys[54]) &&
                      byte > 96 && byte < 123) as u8;
            print!("{}",byte as char);
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
