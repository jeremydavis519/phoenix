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

.include "macros.S"

.global enter_userspace, leave_userspace
.extern get_thread_register_store


.section .trampoline.text.enter_userspace, "ax"

// Switches from kernelspace to userspace. This should only be run in the trampoline, mapped by the
// page tables described by TTBR1_EL1. Calling this through a non-trampoline function pointer will
// result in a crash as soon as the page tables are switched.
// 
// Input:
// X0 = the physical address of the userspace page table
// X1 = the value to restore to SPSR_EL1
// X2 = the address of the thread's next instruction
// X3 = the stack pointer we should use when in the trampoline
// X4 = the address of the `Thread` object that refers to the thread we are switching to
//
// Output (after returning from userspace):
// X0 = a `ThreadStatus` value describing why we left the thread
enter_userspace:
    // Save the kernel's registers on its private stack.
    pushcallee

    // In the case of an exception, we might need access to the current thread's object. We'll also
    // need the return address when we leave this thread.
    stp x4, lr, [sp, -16]!

    // Disable interrupts until we finish the context switch. The ERET instruction will set A, I,
    // and F according to the bits in SPSR.
    msr DAIFSet, 0x7

    // Switch to the trampoline stack and save the kernel's old stack pointer on it so we can find
    // it when we enter kernelspace again.
    mov x19, sp
    mov sp, x3
    str x19, [sp, -16]!

    // Bits 48-51 of the page table address, if they're used, need to be moved to bits 2-5.
    and x19, x0, 0x000f000000000000
    and x0, x0,  0x0000ffffffffffc0
    orr x19, x0, x19, lsr (48 - 2) // X19 = value to put in TTBR0_EL1

    // Prepare the CPU for EL0 (but don't change the root page table yet).
    msr SPSR_EL1, x1
    msr ELR_EL1, x2

    // Restore the thread's registers (except that X19 will be put on the trampoline stack for now).
    mov x0, x4
    ldr x9, =get_thread_register_store
    blr x9
    ldp x30, x9, [x0, 0xf0] // X9 = the thread's version of SP
    msr SP_EL0, x9
    ldp x28, x29, [x0, 0xe0]
    ldp x26, x27, [x0, 0xd0]
    ldp x24, x25, [x0, 0xc0]
    ldp x22, x23, [x0, 0xb0]
    ldp x20, x21, [x0, 0xa0]
    ldp x18, x9, [x0, 0x90] // X9 = the thread's version of X19
    str x9, [sp, -16]!
    ldp x16, x17, [x0, 0x80]
    ldp x14, x15, [x0, 0x70]
    ldp x12, x13, [x0, 0x60]
    ldp x10, x11, [x0, 0x50]
    ldp x8, x9, [x0, 0x40]
    ldp x6, x7, [x0, 0x30]
    ldp x4, x5, [x0, 0x20]
    ldp x2, x3, [x0, 0x10]
    ldp x0, x1, [x0, 0x00]

    // Switch to the thread's root page table and restore the thread's X19.
    msr TTBR0_EL1, x19
    ldr x19, [sp], 16

    eret

.section .text.leave_userspace, "ax"

// Switches from userspace to kernelspace. This is actually called from kernelspace, in response to
// a system call or IRQ, so no context switch is necessary, but instead of returning to the location
// that called `leave_userspace`, it returns to right after the last call to `enter_userspace`.
//
// Input:
// X19 = the `ThreadStatus` that should be passed to the scheduler (determines whether the thread dies
//      or continues running)
// SP is on the trampoline stack, where `enter_userspace` left it, except that the following
//      instructions have been executed with the values the thread left in its registers:
//        STR X30, [SP, -16]!
//        STP X19, X20, [SP, -16]!
// All general-purpose registers except X19, X20, X30, and SP are as the thread left them.
//
// Output (to the caller of `enter_userspace`):
// X0 = the `ThreadStatus` that was passed in through X30
leave_userspace:
    str x19, [sp, -16]!

    cmp x19, 255
    b.eq .Lthread_save_done // Don't bother saving the registers when terminating a thread.

    // Save the thread's registers.
    mov x20, x0 // X20 = the value the thread left in X0
    ldr x0, [sp, 48] // X0 = main kernel stack pointer
    ldr x0, [x0]     // X0 = address of the `Thread` object
    ldr x19, =get_thread_register_store // X0 = address of the thread's register store
    blr x19
    stp x20, x1, [x0, 0x00]
    stp x2, x3, [x0, 0x10]
    stp x4, x5, [x0, 0x20]
    stp x6, x7, [x0, 0x30]
    stp x8, x9, [x0, 0x40]
    stp x10, x11, [x0, 0x50]
    stp x12, x13, [x0, 0x60]
    stp x14, x15, [x0, 0x70]
    stp x16, x17, [x0, 0x80]
    ldp x19, x20, [sp, 16] // X19, X20 = the values the thread left in them
    stp x18, x19, [x0, 0x90]
    stp x20, x21, [x0, 0xa0]
    stp x22, x23, [x0, 0xb0]
    stp x24, x25, [x0, 0xc0]
    stp x26, x27, [x0, 0xd0]
    stp x28, x29, [x0, 0xe0]
    ldr x30, [sp, 32]
    mrs x19, SP_EL0 // X19 = the value the thread left in SP
    stp x30, x19, [x0, 0xf0]
.Lthread_save_done:

    // Remove the thread's registers from the stack, and also retrieve the argument.
    ldr x0, [sp], 48 // X0 = return value

    // Return to where we were on the main stack before entering userspace.
    ldr x9, [sp], 16 // X9 = the stack pointer that was saved on the trampoline stack
    mov sp, x9

    // Enable interrupts (necessary because we're technically in an exception handler and aren't
    // going to execute ERET to leave it).
    msr DAIFClr, 0x7

    // Pop what we pushed onto the main stack before entering userspace.
    ldp x4, lr, [sp], 16
    popcallee

    // Return to the caller of `enter_userspace`.
    ret
