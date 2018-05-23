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
                asm!("push 0" :::: "intel", "volatile"); // push fake error code
                pusha!();
                asm!("
                    mov rdi, rsp // calculate isr context pointer
                    sub rsp, 0
                    call $0
                    add rsp, 8
                    mov rsp, rax // use returned stack pointer
                    ":: "i"($name as extern "C" fn(&'static Context)
                        -> &'static Context)
                    : "rdi" : "intel");
                popa!();
                asm!("
                     add rsp, 8 // pop dummy error code
                     iretq" :::: "intel", "volatile");
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
                    mov rdi, rsp // get context pointer
                    sub rsp, 8 // align stack pointer
                    call $0
                    add rsp, 8 // undo alignment
                    ":: "i"($name as extern "C" fn(&'static Context)
                        -> &'static Context)
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
#[derive(Debug)]
pub struct Context {
    pub regs: Regs,
    pub error_code: usize,
    pub stack_frame: ExceptionStackFrame,
}

#[repr(C)]
#[derive(Debug)]
pub struct Regs {
    r15: usize,
    r14: usize,
    r13: usize,
    r12: usize,
    r11: usize,
    r10: usize,
    r9: usize,
    r8: usize,
    rdi: usize,
    rsi: usize,
    rdx: usize,
    rcx: usize,
    rbx: usize,
    rbp: usize,
    rax: usize,
}

impl Regs {
    pub unsafe fn zero(&mut self) {
        self.r15 = 0;
        self.r14 = 0;
        self.r13 = 0;
        self.r12 = 0;
        self.r11 = 0;
        self.r10 = 0;
        self.r9  = 0;
        self.r8  = 0;
        self.rdi = 0;
        self.rsi = 0;
        self.rdx = 0;
        self.rcx = 0;
        self.rbx = 0;
        self.rbp = 0;
        self.rax = 0;
    }
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
