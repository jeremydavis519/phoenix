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

.global _start

.extern __load_start
.extern __load_end
.extern __bss_end

.section .text.boot.multiboot

// This header is compliant with version 1.6 of the Multiboot Specification.
multiboot_header:
.set MB_MAGIC, 0xe85250d6             // Marks the beginning of the Multiboot header
.Lmagic:         .4byte MB_MAGIC
.set MB_ARCH, 0                       // 32-bit (Protected-Mode) i386
.Larchitecture:  .4byte MB_ARCH
.set MB_LEN, .Lend - multiboot_header // Length of the whole header
.Lheader_length: .4byte MB_LEN
.set MB_CHECKSUM, -(MB_MAGIC + MB_ARCH + MB_LEN)
.Lchecksum:      .4byte MB_CHECKSUM

.Ltags:
// Multiboot information request
.Lmbi:
.2byte 1                 // Type
.2byte 0x1               // Flags (bit 0: optional)
.4byte .Lmbi_end - .Lmbi // Size
// TODO: MBI tags
.Lmbi_end:

// Addresses
.Laddr:
.2byte 2
.2byte 0x0
.4byte .Laddr_end - .Laddr
.4byte multiboot_header // Header address
.4byte __load_start     // Beginning of @progbits segments
.4byte __load_end       // End of @progbits segments
.4byte __bss_end        // End of @nobits segments
.Laddr_end:

// Entry point
.Lentry:
.2byte 3
.2byte 0x0
.4byte .Lentry_end - .Lentry
.4byte _start
.Lentry_end:

// Flags
.Lflags:
.2byte 4
.2byte 0x1
.4byte .Lflags_end - .Lflags
.4byte 0x3 // Console flags (bit 0: supported console required; bit 1: OS has EGA text support)
.Lflags_end:

// Framebuffer (text or graphics)
.Lframebuffer:
.2byte 5
.2byte 0x1
.4byte .Lframebuffer_end - .Lframebuffer
.4byte 80 // Width (characters or pixels, 0 means don't care)
.4byte 25 // Height (characters or pixels, 0 means don't care)
.4byte 0  // Depth (0 for text mode; bits per pixel for graphics mode, 0 means don't care)
.Lframebuffer_end:

// Module alignment (if present, indicates modules must be page-aligned)
// .Lmodalign:
// .2byte 6
// .2byte 0x0
// .4byte .Lmodalign_end - .Lmodalign
// .Lmodalign_end:

.Lend:

.section .text.boot.entry
.code 32

_start:
    // Set up the stack.
    // TODO: This is only done by the first CPU. How should the other CPUs get their stacks? In
    // particular, since stack safety for this CPU is implemented by having the stack right after a
    // read-only page, how can we enforce stack safety for the other CPUs?
    movd stack_end, %esp

    // The Multiboot Specification says the bootloader must enable the A20 line, but we might as
    // well confirm that it actually did it and enable the line ourselves if not.
    call enable_a20

    cmpd 0x36d76289, %eax
    jne .Lno_multiboot
    call multiboot_init
    jmp .Lhas_multiboot
.Lno_multiboot:
    // TODO: If the bootloader wasn't Multiboot-compliant, we'll still need to get a memory map
    // somehow. Our own bootloader will probably be Multiboot-compliant (albeit very simple), so we
    // can probably just panic here.
	movd PANIC_NON_MULTIBOOT
    jmp boot_panic
.Lhas_multiboot:

    // TODO: Finish booting up: replace GDT, make IDT, start paging.
    cli
    hlt

    // TODO: Enter Long Mode.
.code 64

    call kmain

    // kmain shouldn't return, but if it does, just halt.
    movd PANIC_KERNEL_RETURNED, %esi
    jmp boot_panic

.section .text.boot

// Performs the initialization that's required when loaded from a Multiboot-compliant bootloader.
// Preconditions:
//   EBX = Address of the Multiboot information structure.
//   All segments (CS, DS, etc.) have base 0 and limit 0xffffffff.
//   A20 gate is enabled.
//   Paging is not enabled.
//   Interrupts are disabled.
//   GDTR is undefined, so no segment register can safely be written to.
//   IDTR is undefined, so we shouldn't enable interrupts yet.
multiboot_init:
    // TODO: Possibly useful things to get from [EBX]:
    //  - Boot command line (also available in the DTB on other architectures): tag type 1
    //  - Boot loader name (to show the user): tag type 2
    //  - Modules (maybe one that can read the root filesystem): tag type 3
    //  - Memory map: tag type 4 (basic) and type 6 (detailed)
    //  - APM table (for when ACPI isn't available; can probably be found in some other way): tag type 10
    //  - VBE info (for the fallback video driver; can probably be found in some other way): tag type 7
    //  - Framebuffer info (for early text/graphics): tag type 8
    ret

// Enables the A20 line if it hasn't already been enabled. If the line can't be enabled, a note is
// made to remove every odd megabyte from the memory map.
enable_a20:
    // Do nothing if it's already enabled.
    call test_a20
    jnz .Lret

    // The preferred way to enable the gate is to use the BIOS, but we can't do that from Protected
    // Mode. So try the next best thing: the original method, sending commands to the keyboard controller.
    // TODO: Some systems (like the x86 Apple computers) don't have PS/2 keyboard controllers. But those
    // systems do support ACPI. Look for the "8042" flag in the "IA PC Boot Architecture Flags" field of
    // the FADT.
    movb 0xad, %al // Disable first PS/2 port
    outb %al, 0x64
    movb 0xa7, %al // Disable second PS/2 port (if present)
    outb %al, 0x64
    inb 0x60, %al // Flush the output buffer

    call .Lkeyboard_out_wait
    movb 0xd0, %al // Read controller output port
    outb %al, 0x64

    call .Lkeyboard_in_wait
    inb 0x60, %al
    pushl %eax

    call .Lkeyboard_out_wait
    movb 0xd1, %al // Write controller output port
    outb %al, 0x64

    call .Lkeyboard_out_wait
    popl %eax
    orb 2, %al // Enable A20 line
    outb %al, 0x60

    // We could re-enable the ports, but there's no reason to. They'll be enabled when we can handle
    // a keyboard or mouse.

    call test_a20
    jnz .Lret

    // TODO: On some computers, reading a byte from port 0xee enables A20. Is there any way to tell
    // whether this computer supports this method? It's best not to probe ports, even with reads.
    // inb 0xee, %al
    // call test_a20
    // jnz .Lret

    // We've run out of safe options, and the A20 line is still disabled. We just won't allocate
    // anything in the odd megabytes.
    // TODO: Maybe not here, but let the user know that half of the computer's memory is unusable.
    movb [BOOT_INFO + BootInfo_a20_disabled], 1

.Lret:
    ret

.Lkeyboard_out_wait:
    // Wait for the keyboard controller to be ready to receive a byte.
    inb 0x64, %al
    test 2, %al
    pause
    jnz .Lkeyboard_out_wait
    ret

.Lkeyboard_in_wait:
    // Wait for the keyboard controller to send a byte.
    inb 0x64, %al
    test 1, %al
    pause
    jnz .Lkeyboard_in_wait
    ret

// Tests to see if the A20 line is enabled. Clears ZF if so and sets it if not.
test_a20:
    pushl %eax
    // This part of low memory is guaranteed to exist, since it's where the BIOS loads the bootloader.
    movl [0x7dfc], %eax
    cmpl [0x17dfc], %eax
    jne .Lret
    // Equal at first. How about after one is changed?
    notl %eax
    movl %eax, [0x7dfc]
    wbinvd // We're interested in main memory, so flush the cache.
    cmpl [0x17dfc], %eax
    // ZF has been set or cleared appropriately. Undo the change.
    notl [0x7dfc]
.Lret:
    popl %eax
    ret

boot_panic:
    // TODO: Print a message.
    cli
    hlt

.section .stack, "aw", @nobits

.balign 8
stack_start:
.skip 0x10000
stack_end:

.section .data.boot, "aw", @progbits

// This structure corresponds to the `memory::phys::map::BootInfo` struct.
.balign 8
BOOT_INFO:
    .8byte sizeof_BootInfo
    .fill sizeof_BootInfo - 8


.struct 0
BootInfo_size:         .struct BootInfo_size + 8
BootInfo_a20_disabled: .struct BootInfo_a20_disabled + 8
sizeof_BootInfo:
