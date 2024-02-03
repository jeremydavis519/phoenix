/* Copyright (c) 2017-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

MEMORY {
    kernel : ORIGIN = 0x40080000, LENGTH = 16M
}

PHDRS {
    readonly PT_LOAD;
    trampoline PT_LOAD;
    stack PT_LOAD;
    profile PT_LOAD;
    readwrite PT_LOAD;
}

SECTIONS {
    . = 0x40080000;
    __start = .;
    __readonly_start = .;
    __text_start = .;
    .text __start : {
        KEEP(*(.text.boot.entry))
        *(.text.boot) *(.text.boot.*)
        *(.text)
        *(.text.*)
    } >kernel :readonly
    __text_end = .;

    __rodata_start = ADDR(.rodata);
    .rodata : {
        *(.rodata) *(.rodata.*)
    } >kernel
    __rodata_end = .;

    __trampoline_virt = 0xffff000000000000;
    /* The trampoline is physically at this address but will only actually be used at its virtual
     * address. */
    __trampoline_start = ADDR(.trampoline_ro);
    __trampoline_ro_start = ADDR(.trampoline_ro);
    .trampoline_ro BLOCK(0x10000) : {
        *(.trampoline.text.exceptions)
        *(.trampoline.text) *(.trampoline.text.*)
    } >kernel :trampoline
    __trampoline_ro_end = .;
    __trampoline_rw_start = ADDR(.trampoline_rw);
    .trampoline_rw BLOCK(0x10000) : {
        *(.trampoline.data) *(.trampoline.data.*)
    } >kernel
    __trampoline_rw_end = .;
    __trampoline_end = .;
    ROOT_PAGE_TABLE_ADDR_TRAMP = ROOT_PAGE_TABLE_ADDR - __trampoline_start + __trampoline_virt;

    /* This is where the CPUs' private trampoline stacks will be stored in virtual memory. These
     * stacks are only used while the CPUs are executing code in the trampoline; in the main part
     * of the kernel, they have larger stacks where, because they're not mapped in the same address
     * space as userspace, sensitive data can be handled. This address has a page of padding to
     * protect the read-write region below it, then it's rounded up to the next page boundary. */
    __trampoline_stacks_virt = (__trampoline_virt + (__trampoline_end - __trampoline_start) + 0x1ffff) / 0x10000 * 0x10000;

    __readonly_end = .;

    /* The stack is positioned at a 64K boundary after the read-only segments so we can use the
     * read-only (or maybe unmapped) page right below the stack to force an exception on stack
     * overflow. */
    __readwrite_start = __stack_start;
    __rw_nonshareable_start = __stack_start;
    __stack_start = ADDR(.stack);
    .stack BLOCK(0x10000) : {
        *(.stack)
    } >kernel :stack
    __stack_end = .;
    __rw_nonshareable_end = .;

    __rw_shareable_start = __data_start;
    /* The profiler stores its probes in this section. This must exactly fill some integer number of
     * pages because we provide a system call to map them into userspace. */
    __profile_start = ADDR(.profile);
    .profile BLOCK(0x10000) : {
        *(.profile.header)
        . = ALIGN(16);
        __profile_probes_start = .;
        *(.profile)
        __profile_probes_end = .;
        *(.profile.strings)
        . = ALIGN(0x10000);
    } >kernel :profile
    __profile_end = .;

    __data_start = ADDR(.data);
    .data : {
        *(.data) *(.data.*)
    } >kernel :readwrite
    __data_end = .;

    __bss_start = ADDR(.bss);
    .bss : {
        *(.bss) *(.bss.*)
        *(COMMON)
    } >kernel
    __bss_end = .;
    __rw_shareable_end = .;
    __readwrite_end = .;
    __end = .;
}
