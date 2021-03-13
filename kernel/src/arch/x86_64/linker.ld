/* Copyright (c) 2019-2021 Jeremy Davis (jeremydavis519@gmail.com)
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software
 * and associated documentation files (the "Software"), to deal in the Software without restriction,
 * including without limitation the rights to use, copy, modify, merge, publish, distribute,
 * sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or
 * substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
 * NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
 * DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */


ENTRY(_start)

SECTIONS {
	. = 0x100000;
	__start = .;
	__load_start = .;
	__readonly_start = .;
	__text_start = .;
	.text __start : {
		KEEP(*(.text.boot.entry))
		*(.text.boot)
		*(.text)
		*(.text.*)
	}
	__text_end = .;

	__rodata_start = ADDR(.rodata);
	.rodata BLOCK(4096) : {
		*(.rodata)
		*(.rodata.*)
	}
	__rodata_end = .;
	__readonly_end = .;

	/* The stack is positioned at a 64K boundary after .rodata so we can use the read-only (or
	 * maybe unmapped for smaller page sizes) page right below the stack to force an exception
	 * on stack overflow. */
	__readwrite_start = __stack_start;
	__stack_start = ADDR(.stack);
	.stack BLOCK(65536) : {
		*(.stack)
	}
	__stack_end = .;

	__data_start = ADDR(.data);
	.data BLOCK(4096) : {
		*(.data)
		*(.data.*)
	}
	__data_end = .;
	__load_end = .;

	__bss_start = ADDR(.bss);
	.bss BLOCK(4096) : {
		*(.bss)
		*(.bss.*)
	}
	__bss_end = .;
	__readwrite_end = .;
	__end = .;
}

MEMORY {
	kernel : ORIGIN = 0x10_0000, LENGTH = 16M
}
