; Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
; file at the top-level directory of this distribution.
;
; Licensed under the MIT license <LICENSE or
; http://opensource.org/licenses/MIT>, at your option.
; This file may not be copied, modified, or distributed
; except according to those terms.
extern rust_irq_handler

section .text
bits 64
%macro pushall 0
	; Save registers which are normally supposed to
	; be saved by the caller.  I _think_ this list
	; is correct, but don't quote me on that.  I'm
	; probably forgetting something vital.
	push rax
	push rcx
	push rdx
	push rsi
	push rdi
	push r8
	push r9
	push r10
	push r11
%endmacro
%macro popall 0
	pop r11
	pop r10
	pop r9
	pop r8
	pop rdi
	pop rsi
	pop rdx
	pop rcx
	pop rax
%endmacro

%macro isr 1
global isr%1
isr%1:
	pushall
	mov rdi, rsp
	add rdi, 72 ; Account for the pushed registers
	mov rsi, %1
	call irq_common
	popall
	add rsp, 8 ; Pop the error code
	iretq
%endmacro

%macro isr_noerr 1
global isr%1
isr%1:
	push qword 0 ; Push faux error code
	pushall
	mov rdi, rsp
	add rdi, 72 ; Account for the pushed registers
	mov rsi, %1
	call irq_common
	popall
	add rsp, 8 ; Pop the error code
	iretq
%endmacro

isr_noerr 0

isr_noerr 3
isr 13
isr 14
isr_noerr 33

irq_common:
	call rust_irq_handler
	ret
