// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

//!  Code used to handle exceptions (NMIs) and other interrupts. This includes
//!  the keyboard and timer.
//!
//!  Exceptions:
//!
//!  | Name                                   | Vector #   | Type       | Mnemonic | Error Code? |
//!  | -------------------------------------- | ---------- | ---------- | -------- | ----------- |
//!  | Divide by Zero                         | 0  (0x0)   | Fault      | #DE      | No          |
//!  | Debug                                  | 1  (0x1)   | Both       | #DB      | No          |
//!  | Non-maskable Interrupt                 | 2  (0x2)   | Interrupt  | -        | No          |
//!  | Breakpoint                             | 3  (0x3)   | Trap       | #BP      | No          |
//!  | Overflow                               | 4  (0x4)   | Trap       | #OF      | No          |
//!  | Bound Range Exceeded                   | 5  (0x5)   | Fault      | #BR      | No          |
//!  | Invalid Opcode                         | 6  (0x6)   | Fault      | #UD      | No          |
//!  | Device not Available                   | 7  (0x7)   | Fault      | #NM      | No          |
//!  | Double Fault                           | 8  (0x8)   | Abort      | #DF      | No          |
//!  | <del>Coprocessor Segment Overrun</del> | 9  (0x9)   | Fault      | -        | No          |
//!  | Invalid TSS                            | 10 (0xA)   | Fault      | #TS      | Yes         |
//!  | Segment not Present                    | 11 (0xB)   | Fault      | #NP      | Yes         |
//!  | Stack-Segment Fault                    | 12 (0xC)   | Fault      | #SS      | Yes         |
//!  | General Protection Fault               | 13 (0xD)   | Fault      | #GP      | Yes         |
//!  | Page Fault                             | 14 (0xE)   | Fault      | #PF      | Yes         |
//!  | Reserved                               | 15 (0xF)   | -          | -        | No          |
//!  | x87 Floating Point Exception           | 16 (0x10)  | Fault      | #MF      | No          |
//!  | Alignment Check                        | 17 (0x11)  | Fault      | #AC      | Yes         |
//!  | Machine Check                          | 18 (0x12)  | Fault      | #MC      | No          |
//!  | SIMD Floating-Point Exception          | 19 (0x13)  | Fault      | #XM/#XF  | No          |
//!  | Virtualisation Exception               | 20 (0x14)  | -          | #VE      | No          |
//!  | Reserved                               | 21 (0x15)  | -          | -        | No          |
//!  | Security Exception                     | 22 (0x16)  | -          | #SX      | Yes         |
//!  | Reserved                               | 23 (0x17)  | -          | -        | No          |
//!  | Triple Fault                           | 24 (0x15)  | -          | -        | No          |
//!  | FPU Error Interrupt                    | 25 (0x18)  | Interrupt  | #FERR    | No          |
//!
//!
//!  Syscalls/IRQs:
//!
//!  | Name      | Vector #   | Type     | Arguments/Misc             |
//!  | --------- | ---------- | -------- | -------------------------- |
//!  | Timer     | 32 (0x20)  | IRQ (M)  | Data to read from keyboard |
//!  | Keyboard  | 33 (0x21)  | IRQ (M)  | Data to read from keyboard |
//!  | Yield     | 34 (0x22)  | Syscall  | `rax` == 0                 |
//!  | Sleep     | 34 (0x22)  | Syscall  | Time to sleep is `rax`     |
//!  | Exit      | 35 (0x23)  | Syscall  | None                       |

#![allow(dead_code)]
#![allow(unreachable_code)]

use spin::{Mutex, Once};

use x86_64::VirtualAddress;
use x86_64::registers::{self, flags};
use x86_64::structures::tss::TaskStateSegment;

use self::idt::Idt;
use self::gdt::Gdt;

use sync::IrqLock;
use scheduler;

use memory;

use self::pic::ChainedPICs;
pub use self::keyboard::KEYBOARD;

pub use self::context::Context;

/// Abstraction of the PS/2 keyboard
mod keyboard;
/// The programmable interrupt controller
mod pic;
/// Abstraction of the Global Descriptor Table
mod gdt;
/// Abstraction of the Interrupt Descriptor Table
mod idt;
/// Interrupt context handling in Rust
#[macro_use]
mod context;

/// Enable Interrupts
#[inline]
pub unsafe fn enable() {
    asm!("sti");
}

/// Disable Interrupts
#[inline]
pub unsafe fn disable() {
    asm!("cli");
}

pub fn enabled() -> bool {
    flags::flags().contains(flags::Flags::IF)
}

/// This is the Interrupt Descriptor Table that contains handlers for all
/// interrupt vectors that we support. Each handler is set in its initialization
/// and is not modified again.
// FIXME make CPU local
static IDT: IrqLock<Idt> = IrqLock::new(Idt::new());

/// The Rust interface to the 8086 Programmable Interrupt Controller
pub static PIC: Mutex<ChainedPICs> = Mutex::new(unsafe { ChainedPICs::new(0x20, 0x28) });

const DF_TSS_INDEX: u16 = 0;
#[cfg(feature = "test")]
const TEST_TSS_INDEX: u16 = 1;

pub const SLEEP_INT: u8 = 0x22;
pub const EXIT_INT: u8 = 0x23;

/// Static Task State Segment
static TSS: Once<TaskStateSegment> = Once::new();
/// Static Gdt
static GDT: Once<Gdt> = Once::new();

pub fn init() {
    // Set up the TSS
    let tss = TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();

        let double_fault_stack = memory::alloc_stack(1)
            .expect("Could not allocate double fault stack");

        tss.interrupt_stack_table[DF_TSS_INDEX as usize] =
            VirtualAddress(double_fault_stack.top());

        #[cfg(feature = "test")] {
            let test_stack = memory::alloc_stack(1)
                .expect("Could not allocate test stack");
            tss.interrupt_stack_table[TEST_TSS_INDEX as usize] =
                VirtualAddress(test_stack.top());
        }

        tss
    });

    // Set up the GDT with a code segment and TSS segment and then load both
    // segments
    use x86_64::structures::gdt::SegmentSelector;
    use x86_64::instructions::segmentation::set_cs;
    use x86_64::instructions::tables::load_tss;
    let mut code_selector = SegmentSelector(0);
    let mut tss_selector = SegmentSelector(0);
    let gdt = GDT.call_once(|| {
        let mut gdt = gdt::Gdt::new();
        code_selector =
            gdt.add_entry(gdt::Descriptor::kernel_code_segment());
        tss_selector =
            gdt.add_entry(gdt::Descriptor::tss_segment(&tss));
        gdt
    });
    gdt.load();

    unsafe {
        // Reload code segment register
        set_cs(code_selector);
        // load TSS
        load_tss(tss_selector);
    }

    // Set up the IDT
    let mut idt = IDT.lock();

    // Initialize handlers
    idt.set_handler(0x0, handler!(de_handler));
    idt.set_handler(0x3, handler!(breakpoint_handler));
    unsafe {
        // Use another stack to prevent triple faults
        idt.set_handler(0x8, handler_error_code!(df_handler))
            .set_stack_index(DF_TSS_INDEX);
    }
    idt.set_handler(0xD, handler_error_code!(gp_handler));
    idt.set_handler(0xE, handler_error_code!(pf_handler));
    // PIC handlers
    idt.set_handler(0x20, handler!(timer_handler));
    idt.set_handler(0x21, handler!(kb_handler));
    idt.set_handler(SLEEP_INT, handler!(sleep_handler));
    idt.set_handler(EXIT_INT, handler!(exit_handler));

    // Set up the PIC and initialize interrupts.
    unsafe {
        idt.load();
        {
            let mut pic = PIC.lock();
            pic.initialize();
        }
        enable();
    }
}

/// Divide by zero handler
///
/// Occurs when the hardware attempts to divide by zero. Unrecoverable.
extern "C" fn de_handler(c: &'static Context) -> &'static Context {
    panic!("EXCEPTION DIVIDE BY ZERO\n{:#?}", c.stack_frame);
    c
}

/// Breakpoint handler
///
/// A harmless interrupt, operation is safely resumed after printing a message.
extern "C" fn breakpoint_handler(c: &'static Context) -> &'static Context {
    println!("Breakpoint at {:#?}\n{:#?}",
             (c.stack_frame).instruction_pointer,
             c.stack_frame);
    c
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
extern "C" fn df_handler(c: &'static Context) -> &'static Context {
    panic!("\nEXCEPTION: DOUBLE FAULT\n{:#?}", c.stack_frame);
    c
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
extern "C" fn gp_handler(c: &'static Context) -> &'static Context {
    panic!("EXCEPTION GENERAL PROTECTION FAULT\nerror_code: {}\n{:#?}\n", c.error_code, c.stack_frame);
    c
}

/// Page Fault handler
///
/// A Page Fault occurs when:
/// + A page directory or table entry is not present in physical memory.
/// + Attempting to load the instruction tlb with an address for a
/// non-executable page.
/// + A protection check (privileges, read/write) failed.
/// + A reserved bit in the page directory or table entries is set to 1.
extern "C" fn pf_handler(context: &'static Context) -> &'static Context {
    panic!("EXCEPTION PAGE FAULT\nerror_code: 0b{:b}\nAddress that caused the fault: {:#?}\n{:#?}",
           context.error_code, registers::control_regs::cr2(), context.stack_frame);
    context
}

/// Timer handler
extern "C" fn timer_handler(c: &'static Context) -> &'static Context {
    unsafe {
        PIC.lock().master.end_of_interrupt();
    }
    scheduler::tick(c)
}

/// Keyboard handler
///
/// This function pages the `Keyboard` port to get the key that was pressed, it then
/// prints the associated byte to the screen and saves the state of the keyboard.
extern "C" fn kb_handler(c: &'static Context) -> &'static Context {
    let mut kb = KEYBOARD.lock();
    match kb.port.read() {
        // If the key was just pressed,
        // then the top bit of it is unset
        x if x & 0x80 == 0 => {
            kb.keys[x as usize] = true;
            let mut byte = kb.kbmap[x as usize];

            // If either shift is pressed, make it
            // capital.
            byte = if kb.keys[42] || kb.keys[54] {
                match byte {
                    b if b >= b'a' && b <= b'z' => b - 0x20,

                    b'1' => b'!',
                    b'2' => b'@',
                    b'3' => b'#',
                    b'4' => b'$',
                    b'5' => b'%',
                    b'6' => b'^',
                    b'7' => b'&',
                    b'8' => b'*',
                    b'9' => b'(',
                    b'0' => b')',

                    b'`' => b'~',
                    b'-' => b'_',
                    b'=' => b'+',
                    b'[' => b'{',
                    b']' => b'}',
                    b'\\'=> b'|',
                    b';' => b':',
                    b'\''=> b'\"',
                    b',' => b'<',
                    b'.' => b'>',

                    _ => b'\0',
                }
            } else {
                byte
            };
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
    c
}

extern "C" fn sleep_handler(c: &'static Context) -> &'static Context {
    let time = c.regs.rax;
    if time == 0 {
        scheduler::sched_yield(c)
    } else {
        scheduler::sched_sleep(c, time as u8)
    }
}

extern "C" fn exit_handler(c: &'static Context) -> &'static Context {
    scheduler::sched_exit(c)
}

#[cfg(feature = "test")]
pub mod tests {
    use tap::TestGroup;

    /// Returns true iff a block raises a certain interrupt.
    macro_rules! assert_throws {
        ($vector:expr, $func:block) => {{
            // Disallow recursive expansions
            #[allow(unused_macros)]
            macro_rules! assert_throws {
                () => ()
            }

            use scheduler;
            use core::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
            use super::context::Context;

            // XXX we don't have `thread_wait()`, so wait on a thread through
            // an atomic int
            static WAIT: AtomicUsize = ATOMIC_USIZE_INIT;
            const FAILURE_MAGIC: usize = 2;
            const SUCCESS_MAGIC: usize = 1;

            // Run the block in a new thread
            #[allow(unused_unsafe)]
            extern "C" fn interrupt_thread() {
                unsafe {
                    $func
                }
                // The block has fallen through, indicate failure.
                WAIT.store(FAILURE_MAGIC, Ordering::Release);
                // A return is an implicit exit
            }
            extern "C" fn tmp_handler(c: &'static Context) -> &'static Context {
                WAIT.store(SUCCESS_MAGIC, Ordering::Release);
                scheduler::sched_exit(c)
            }

            WAIT.store(0, Ordering::Release);

            // If this fails, $vector is not a valid expression
            let int: u8 = $vector;

            // Get the old function & TSS index, and set the new ones
            let old_handler;
            let old_tss;
            {
                let mut idt = super::IDT.lock();
                old_handler = idt.get_handler(int).func();
                old_tss = idt.get_handler(int).options().get_stack_index();
                unsafe {
                    match int {
                        // Error code vectors
                        0xA ... 0xE | 0x11 | 0x16 =>
                            idt.set_handler(int, handler_error_code!(tmp_handler)),
                        _ => idt.set_handler(int, handler!(tmp_handler)),
                    }.set_stack_index(super::TEST_TSS_INDEX)
                };
            }

            scheduler::add(interrupt_thread)
                .expect("Could not generate a new thread to test interrupts");

            // spin until the block is complete or interrupted
            let mut res;
            while {
                res = WAIT.load(Ordering::Acquire);
                res == 0
            } {
                scheduler::thread_yield();
            }

            // Restore the old handler & TSS if they existed and hope that
            // nothing interrupted into the new handler except for the thread
            // we just ran >…<
            match (old_handler, old_tss) {
                (Some(handler), Some(index)) => {
                    unsafe {
                        super::IDT.lock().set_handler(int, handler)
                            .set_stack_index(index)
                    };
                },

                (Some(handler), None) => {
                    super::IDT.lock().set_handler(int, handler);
                }

                _ => (),
            }

            if res == FAILURE_MAGIC {
                false
            } else if res == SUCCESS_MAGIC {
                true
            } else {
                panic!("Error in assert_throws!()");
            }
        }}
    }

    pub fn run() {
        test_interrupts();
        //test_no_interrupts();
    }

    fn test_interrupts() {
        let mut tap = TestGroup::new(6);
        tap.diagnostic("Testing interrupts");

        tap.assert_tap(assert_throws!(0x0, {
            asm!("
                 xor rcx, rcx
                 div rcx" ::: "rax","rcx" : "intel", "volatile");
        }), "#DE not caught from a divide by zero");

        tap.assert_tap(assert_throws!(0x3, {
            asm!("int 3" :::: "intel", "volatile");
        }), "#DB not caught after debug interrupt!");

        tap.assert_tap(assert_throws!(0x6, {
            asm!("ud2" :::: "intel", "volatile");
        }), "#UD not caught after an invalid opcode execution");

        tap.assert_tap(assert_throws!(0x8, {
            #[allow(unconditional_recursion)]
            fn stack_overflow() {
                stack_overflow();
            }
            stack_overflow();
        }), "#DF not caught after a stack overflow");

        tap.assert_tap(assert_throws!(0xD, {
            asm!("
                mov rbx, 0xfeedbeefbaadf00d
                mov [rbx], rax"
                ::: "rbx" : "intel", "volatile");
        }), "#GP not caught after a noncanonical address dereference");

        tap.assert_tap(assert_throws!(0xE, {
            asm!("
                xor rbx, rbx
                mov [rbx], rax"
                ::: "rbx" : "intel", "volatile");
        }), "#PF not caught after a NULL dereference");
    }

    fn test_no_interrupts() {
        let mut tap = TestGroup::new(0x18);
        tap.diagnostic("Making sure exceptions are not happening");
        for i in 0..0x18 {
            tap.assert_tap(!assert_throws!(i, {
                // do some ordinary stuff
                let mut _a = 0;
                for i in 0..50 {
                    _a += i;
                }
            }), "Exception was thrown for no reason");
        }
    }
}
