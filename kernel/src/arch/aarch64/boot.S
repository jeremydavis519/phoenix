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

.global _start
.global boot_panic
.global BOOT_INFO
.extern __bss_start, __bss_end
.extern init_interrupts
.extern kmain
.extern puts
.extern record_generic_timer_frequency

.section .text.boot.entry

// The kernel's entry point.
// Parameters: w0 = 32-bit address of the device tree block (DTB).
//             x1 = the machine type (defined in linux/arch/arm/tools/mach-types), or all ones
//                     to indicate a device-tree-only (DT-only) platform
//             x2 = the address of the ATAGs (or the DTB?)
_start:
    // Enable stack alignment checks for EL1. Whenever the kernel uses the stack, it will have to
    // be aligned at a 16-byte boundary, or else an SP alignment fault exception will occur.
    mrs x9, SCTLR_EL1
    orr x9, x9, 0b1000 // SCTLR_EL1.SA
    msr SCTLR_EL1, x9

    // Set up the stack.
    // FIXME: Set up a different stack for each CPU. These instructions will give you this CPU's ID:
    // mrs x_, MPIDR_EL1
    // and x_, x_, 0xff // Apparently just the lowest affinity level is enough?
    // Unfortunately, this will break the stack safety for all but CPU 0, since it currently
    // depends on having the stack right after a read-only page. How can we efficiently enforce
    // stack safety for the other CPUs?
    msr SPSel, 1
    ldr x9, =stack_end
    sub sp, x9, #16
    mov fp, sp
    stp xzr, xzr, [fp] // Set up a null frame record to signal when stack unwinding should end. FP = 0, LR = 0

    // Record the boot info before we lose it.
    ldr x9, =BOOT_INFO
    str x0, [x9, BootInfo_dtb]

    // Allow floating point and SIMD instructions.
    bl init_fp_simd

    // The rest of the code will assume we're running in EL1. If that's not the case, we have to do
    // some initialization in EL2 or EL3 first.
    mrs x9, CurrentEL
    and x9, x9, 0b1100 // Everything except bits 2 and 3 is reserved.
    cmp x9, 1 << 2
    b.eq .Lel1 // Already at EL1
    ldr x0, =PANIC_EL0
    b.lt boot_panic // Somehow at EL0
    cmp x9, 2 << 2
    b.eq .Lel2
.Lel3:
    bl el3_init
.Lel2:
    bl el2_init
    // TODO: Switch to EL1.
.Lel1:

    // Clear the bss section.
    ldr x0, =__bss_start // x0 = base
    mov x1, #0           // x1[0..8] = byte to set everywhere
    ldr x2, =__bss_end
    sub x2, x2, x0       // x2 = length
    bl memset

    // Set up virtual address translation.
    bl init_paging

    // Set up exception handlers and enable exceptions.
    bl init_exceptions

    // Jump to Rust.
    bl kmain

    // kmain shouldn't return, but if it does, just halt.
    ldr x0, =PANIC_KERNEL_RETURNED
    b boot_panic

.section .text.boot

// Initializes the EL3 system registers to allow the kernel to run at EL1.
el3_init:
    ldr x0, =PANIC_EL3_NOT_SUPPORTED
    b boot_panic

// Initializes the EL2 system registers to allow the kernel to run at EL1.
el2_init:
    ldr x0, =PANIC_EL2_NOT_SUPPORTED
    b boot_panic

// Initializes the control registers for floating-point and SIMD instructions
// and enables those instructions for both EL1 and EL0.
init_fp_simd:
    // We need to enable the instructions first, since that also enables access to the control registers.
    // Bit 28 (TTA) = 0: Access to trace registers in EL1 and EL0 is untrapped.
    // Bits 20-21 (FPEN) = 11: Access to floating point, Advanced SIMD, and SVE instructions is untrapped.
    // Bits 16-17 (ZEN) = 11: Access to SVE instructions is untrapped.
    ldr x9, =0x00330000
    msr CPACR_EL1, x9
    isb

    // Floating Point Control Register: Determines how floating-point arithmetic works.
    // All zeroes corresponds to following the IEEE 754 standard in round-toward-nearest mode
    // with untrapped exception handling (i.e. an FP exception automatically sets a bit of FPSR to 1).
    msr FPCR, xzr

    // Make sure the MSR instructions finish before running more code that might include FP or SIMD instructions.
    isb

    ret

// Panics and halts the boot process with the null-terminated error message found at [X0].
boot_panic:
    // Print the panic message.
    mov x9, x0
    ldr x0, =PANIC_GENERAL
    bl puts
    mov x0, x9
    bl puts

    // Halt.
    msr DAIFSet, #0xF
    dsb sy
.Lhalt:
    wfi
    b .Lhalt

// This isn't in a .bss section because we clear all of those in a subroutine.
.section .stack, "aw", @nobits

.balign 16
stack_start:
.skip 0x100000
stack_end:

.section .data.boot, "aw", @progbits

// This structure corresponds to the `memory::phys::map::BootInfo` struct.
.balign 8
BOOT_INFO:
    .8byte sizeof_BootInfo
    .fill sizeof_BootInfo - 8

.section .rodata.boot, "a", @progbits

// TODO: Define these in the i18n crate as extern statics so they aren't locked to English.
PANIC_GENERAL:           .asciz "kernel panicked: "
PANIC_EL0:               .asciz "cannot run at Exception Level 0"
PANIC_KERNEL_RETURNED:   .asciz "returned from kmain"
PANIC_EL3_NOT_SUPPORTED: .asciz "running at Exception Level 3 not yet supported"
PANIC_EL2_NOT_SUPPORTED: .asciz "running at Exception Level 2 not yet supported"


.struct 0
BootInfo_size:  .struct BootInfo_size + 8
BootInfo_dtb:   .struct BootInfo_dtb + 8
sizeof_BootInfo:
