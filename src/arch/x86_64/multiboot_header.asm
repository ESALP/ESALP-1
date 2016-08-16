; Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
; file at the top-level directory of this distribution.
;
; Licensed under the MIT license <LICENSE or
; http://opensource.org/licenses/MIT>, at your option.
; This file may not be copied, modified, or distributed
; except according to those terms.

section .multiboot_header
header_start:
	dd 0xe85250d6					;Multiboot2 magic number
	dd 0							;Run in protected i386 mode
	dd header_end - header_start	;header length
	;check sum
	dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))

	;optional tags

	;end tags
	dw 0	;type
	dw 0	;flags
	dd 8	;size
header_end:
