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
		PROVIDE_HIDDEN(__code_start = .);
		*(.text .text.*)
		PROVIDE_HIDDEN(__code_end = .);
	}

	.data ALIGN(4K) : AT(ADDR(.data) - KERNEL_BASE)
	{
		PROVIDE_HIDDEN(__data_start = .);
		*(.padata)
		*(.data .data.* )
		PROVIDE_HIDDEN(__data_end = .);
	}

	.bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_BASE)
	{
		PROVIDE_HIDDEN(__bss_start = .);
		*(.bss .bss.*)
		PROVIDE_HIDDEN(__bss_end = .);
	}

	.rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_BASE)
	{
		PROVIDE_HIDDEN(__rodata_start  = .);
		*(.rodata .rodata.*)
		*(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
		PROVIDE_HIDDEN(__rodata_end = .);
	}

	.got ALIGN(4K) : AT(ADDR(.got) - KERNEL_BASE)
	{
		*(.got)
	}

	.got.plt ALIGN(4K) : AT(ADDR(.got.plt) - KERNEL_BASE)
	{
		*(.got.plt)
	}

	.debug ALIGN(4K) : AT(ADDR(.debug) - KERNEL_BASE)
	{
		*(.debug)
	}


	.gcc_except_table ALIGN(4K) : AT(ADDR(.gcc_except_table) - KERNEL_BASE)
	{
		*(.gcc_except_table)
	}
}
