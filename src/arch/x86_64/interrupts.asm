; Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
; file at the top-level directory of this distribution.
;
; Licensed under the MIT license <LICENSE or
; http://opensource.org/licenses/MIT>, at your option.
; This file may not be copied, modified, or distributed
; except according to those terms.
extern KEXIT
extern rust_de_interrupt_handler
extern rust_pf_interrupt_handler
extern rust_keyboard_interrupt_handler

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

global sti
sti:
	sti
	ret

global irq0
irq0:
	mov rdi, rsp
	; Call a Rust function.
	call rust_de_interrupt_handler

global irqE
irqE:
	mov rdi, rsp
	; Call a Rust function.
	call rust_pf_interrupt_handler

global irq21
irq21:
	pushall

	; Call a Rust function.
	call rust_keyboard_interrupt_handler

	popall

	iretq
