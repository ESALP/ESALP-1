/*
 * Copyright 2016 JJ Garzella and Calvin Lee. See the README.md
 * file at the top-level directory of this distribution.
 *
 * Licensed under the MIT license <LICENSE or
 * http://opensource.org/licenses/MIT>, at your option.
 * This file may not be copied, modified, or distributed
 * except according to those terms.
 */

ENTRY(start)

SECTIONS {
	. = 1M;

	/* .rodata is at the beginning because we don't want to
	 * waste 4k on just a few bytes. This will _have_ to change
	 * when switching to a higher half kernel because the header
	 * and boot code will have to be in one section, while rust
	 * code/data is in the rest.
	 */
	.rodata :
	{
		/* ensure multiboot header will be at the beginning */
		KEEP(*(.multiboot_header))
		*(.rodata .rodata.*)
		. = ALIGN(4K);
	}

	.text :
	{
		*(.text .text.*)
		. = ALIGN(4K);
	}

	.data :
	{
		*(.data .data.* )
		. = ALIGN(4K);
	}

	.bss :
	{
		*(.bss .bss.*)
		. = ALIGN(4K);
	}

	.got :
	{
		*(.got)
		. = ALIGN(4K);
	}

	.got.plt :
	{
		*(.got.plt)
		. = ALIGN(4K);
	}

	.data.rel.ro : ALIGN(4K)
	{
		*(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
		. = ALIGN(4K);
	}

	.gcc_except_table : ALIGN(4K)
	{
		*(.gcc_except_table)
		. = ALIGN(4K);
	}
}
