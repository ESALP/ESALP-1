; Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
; file at the top-level directory of this distribution.
;
; Licensed under the MIT license <LICENSE or
; http://opensource.org/licenses/MIT>, at your option.
; This file may not be copied, modified, or distributed
; except according to those terms.

global start

section .text
bits 32 ;We are still in protected mode
start:
	;Print 'OK' to the screen
	mov dword [0xb8000], 0x2f4b2f4f
	hlt
