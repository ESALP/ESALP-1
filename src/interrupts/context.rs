// Copyright 2016 Phillip Oppermann, Calvin Lee and JJ Garzella.
// See the README.md file at the top-level directory of this
// distribution.
//
// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.
#![allow(unused_macros)]

use core::fmt;

macro_rules! pusha {
    () => {
        asm!("
            // Push all regs
            push rax
            push rbp
            push rbx
            push rcx
            push rdx
            push rsi
            push rdi
            push r8
            push r9
            push r10
            push r11
            push r12
            push r13
            push r14
            push r15
            ":::: "intel", "volatile");
    }
}

macro_rules! popa {
    () => {
        asm!("
            pop r15
            pop r14
            pop r13
            pop r12
            pop r11
            pop r10
            pop r9
            pop r8
            pop rdi
            pop rsi
            pop rdx
            pop rcx
            pop rbx
            pop rbp
            pop rax
            ":::: "intel", "volatile");
    }
}

macro_rules! handler {
    ($name: ident) => {{
        #[naked]
        extern "C" fn wrapper() {
            unsafe {
                pusha!();
                asm!("
                    lea rdi, [rsp + 15 * 8] // calculate exception frame pointer
                    call $0
                    ":: "i"($name as extern "C" fn (&ExceptionStackFrame))
                    : "rdi" : "intel");
                popa!();
                asm!("iretq" :::: "intel", "volatile");
                ::core::intrinsics::unreachable();
            }
        }
        wrapper
    }}
}
macro_rules! handler_error_code {
    ($name: ident) => {{
        #[naked]
        extern "C" fn wrapper() {
            unsafe {
                pusha!();
                asm!("
                    lea rdi, [rsp + 16 * 8] // get frame pointer
                    mov rsi, [rsp + 15 * 8] // get error code
                    sub rsp, 8 // align stack pointer
                    call $0
                    add rsp, 8 // undo alignment
                    ":: "i"($name as extern "C" fn (&ExceptionStackFrame,u64))
                    : "rdi" : "intel");
                popa!();
                asm!("
                     add rsp, 8 // pop error code
                     iretq" :::: "intel", "volatile");
                ::core::intrinsics::unreachable();
            }
        }
        wrapper
    }}
}
#[repr(C)]
pub struct ExceptionStackFrame {
    pub instruction_pointer: usize,
    pub code_segment: usize,
    pub cpu_flags: usize,
    pub stack_pointer: usize,
    pub stack_segment: usize,
}
impl fmt::Debug for ExceptionStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct Hex(usize);
        impl fmt::Debug for Hex {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{:#x}", self.0)
            }
        }

        let mut s = f.debug_struct("ExceptionStackFrame");
        s.field("instruction_pointer", &Hex(self.instruction_pointer));
        s.field("code_segment", &self.code_segment);
        s.field("cpu_flags", &Hex(self.cpu_flags));
        s.field("stack_pointer", &Hex(self.stack_pointer));
        s.field("stack_segment", &self.stack_segment);
        s.finish()
    }
}
