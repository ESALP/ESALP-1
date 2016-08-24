; Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
; file at the top-level directory of this distribution.
;
; Licensed under the MIT license <LICENSE or
; http://opensource.org/licenses/MIT>, at your option.
; This file may not be copied, modified, or distributed
; except according to those terms.

extern KEXIT
; We follow the System V calling conventions, which rust uses, in order to
; get and return arguments. In general, all calling arguments are passed in
; rdi, rsi, rdx, rcx( or r10?), r8 and r9 or varients thereof (the first 32
; bit argument will be passed in edi, the first 16 in di, and the first 8 in
; di as well) and the return value is passed in rax.
; All registers except RBP, RBX, and r12-r15 are caller preserved :)

; Here we define the in and out functions we will use for interrupts
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

; Error puts function for long mode, if we
; ever need to extend the file to need it
; result: printf("ERROR: %s",rdi);
global eputs
; Regular puts, is called with a pointer
; to a string and a color byte.
global puts
eputs:
	;0x04, red on black.
	mov rax, 0x044F045204520445
	mov [0xb8000], rax
	mov rax, 0x00000420043a0452
	mov [0xb8008], rax
	;prepare to "call" puts
	mov si, 0x04
	push KEXIT ;Makes puts ret to KEXIT
puts:
	mov rcx, 0xb800e
	mov dx, si
.loop:
	mov al, [rdi]

	test al, al
	jz .end

	;char
	mov byte [rcx], al
	inc rcx
	;color
	mov byte [rcx], dl
	inc rcx
	inc rdi
	jmp .loop
.end:
	xor eax, eax
	ret
