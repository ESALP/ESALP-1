/* Copyright 2017 Phillip Oppermann, Calvin Lee and JJ Garzella.
 * See the README.md file at the top-level directory of this
 * distribution.
 *
 * Licensed under the MIT license <LICENSE or
 * http://opensource.org/licenses/MIT>, at your option.
 * This file may not be copied, modified, or distributed
 * except according to those terms.
*/

ENTRY(start)
OUTPUT_FORMAT(elf64-x86-64)

KERNEL_BASE = 0xFFFFFFFF80000000;

SECTIONS {

	. = 1M;

	.init ALIGN(4K) :
	{
		KEEP(*(.multiboot_header))
		*(.init .init.*)
	}

	. += KERNEL_BASE;

	.text ALIGN(4K) : AT(ADDR(.text) - KERNEL_BASE)
	{
		*(.text .text.*)
	}

	.data ALIGN(4K) : AT(ADDR(.data) - KERNEL_BASE)
	{
		*(.padata)
		*(.data .data.* )
	}

	.bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_BASE)
	{
		*(.bss .bss.*)
	}

	.rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_BASE)
	{
		*(.rodata .rodata.*)
	}

	.data.rel.ro ALIGN(4K) : AT(ADDR(.data.rel.ro) - KERNEL_BASE)
	{
		*(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
	}


	/DISCARD/ : {
		*(.gcc_except_table)
		*(.debug*)
	}
}
