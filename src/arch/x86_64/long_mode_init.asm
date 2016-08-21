; Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
; file at the top-level directory of this distribution.
;
; Licensed under the MIT license <LICENSE or
; http://opensource.org/licenses/MIT>, at your option.
; This file may not be copied, modified, or distributed
; except according to those terms.

global long_mode_start
global KEXIT

extern rust_main

section .text
bits 64
long_mode_start:
	; call Rust
	call rust_main

	; rust main returned, print `OS returned!`
	mov rax, 0x4f724f204f534f4f
	mov [0xb8000], rax
	mov rax, 0x4f724f754f744f65
	mov [0xb8008], rax
	mov rax, 0x4f214f644f654f6e
	mov [0xb8010], rax
	hlt

	; If the system has nothing more to do, put the computer into an
	; infinite loop. To do that:
	; 1) Disable interrupts with cli (clear interrupt enable in eflags).
	;    They are already disabled by the bootloader, so this is not needed.
	;    Mind that you might later enable interrupts and return from
	;    kernel_main (which is sort of nonsensical to do).
	; 2) Wait for the next interrupt to arrive with hlt (halt instruction).
	;    Since they are disabled, this will lock up the computer.
	; 3) Jump to the hlt instruction if it ever wakes up due to a
	;    non-maskable interrupt occurring or due to system management mode.
KEXIT:
	cli
.loop:
	hlt
	jmp .loop

; Here we define the in and out functions we will use for interrupts

; We follow the System V calling conventions, which rust uses, in order to
; get and return arguments. In general, all calling arguments are passed in
; rdi, rsi, rdx, rcx( or r10?), r8 and r9 or varients thereof (the first 32
; bit argument will be passed in edi, the first 16 in di, and the first 8 in
; di as well) and the return value is passed in rax.
; All registers except RBP, RBX, and r12-r15 are caller preserved :)
global inb
inb:
	mov dx, di
	in al, dx
	ret

global outb
outb:
	mov dx, di
	mov ax, si
	out dx, al
	ret

global inw
inw:
	mov dx, di
	in ax, dx
	ret

global outw
outw:
	mov dx, di
	mov ax, si
	out dx, ax
	ret

global inl
inl:
	mov dx, di
	in eax, dx
	ret

global outl
outl:
	mov dx, di
	mov eax, esi
	out dx, eax
	ret
