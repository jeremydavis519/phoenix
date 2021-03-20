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

.section .text.boot

.extern __trampoline_start, __trampoline_virt
.extern ROOT_PAGE_TABLE_ADDR_TRAMP
.extern aarch64_handle_synchronous_exception, aarch64_handle_irq
.extern leave_userspace
.global init_exceptions

.include "macros.S"

// Initializes the Vector Based Address Registers and enables exceptions.
init_exceptions:
    ldr x9, =exceptions_el1
    ldr x10, =__trampoline_start
    ldr x11, =__trampoline_virt
    sub x9, x9, x10
    add x9, x9, x11
    msr VBAR_EL1, x9

    msr DAIFClr, 0xf // Enable all exceptions.
    ret

.section .trampoline.text.exceptions, "ax"

.balign 0x800
exceptions_el1:
synchronous_el1_sp0:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x9, x30 // X19 = trampoline stack pointer

    stp x6, x7, [sp, -16]!
    stp x0, x1, [sp, -16]!
    mov x0, 0       // Current thread (None)
    mrs x1, ESR_EL1 // Syndrome register
    mov x6, sp      // Where to put the return value, if any
    mov x7, 1       // Exception level
    bl aarch64_handle_synchronous_exception_indirect
    ldp x0, x1, [sp], 16
    ldp x6, x7, [sp], 16

    msr TTBR0_EL1, x20

    mov sp, x19
    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Ls10end:
.if .Ls10end - synchronous_el1_sp0 > 0x80
    .err
.endif

.balign 0x80
irq_el1_sp0:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    bl aarch64_handle_irq_indirect

    msr TTBR0_EL1, x20

    mov sp, x19
    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Li10end:
.if .Li10end - irq_el1_sp0 > 0x80
    .err
.endif

.balign 0x80
fiq_el1_sp0:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    str x9, [sp, -16]!
    ldr x9, =aarch64_handle_irq
    blr x9
    ldr x9, [sp], 16

    msr TTBR0_EL1, x20

    mov sp, x19
    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Lf10end:
.if .Lf10end - fiq_el1_sp0 > 0x80
    .err
.endif

.balign 0x80
serror_el1_sp0:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    // ****** TODO
    str x9, [sp, -16]!
    mov x9, '#'
    bl print_char
    ldr x9, [sp], 16
    // ******

    msr TTBR0_EL1, x20

    mov sp, x19
    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Le10end:
.if .Le10end - serror_el1_sp0 > 0x80
    .err
.endif

.balign 0x80
synchronous_el1_spx:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    // We would just use `pushtemp` and `poptemp`, but we need the order of operations to be slightly different.
    stp x30, x1, [sp, -16]!
    stp x2, x3, [sp, -16]!
    stp x4, x5, [sp, -16]!
    stp x6, x7, [sp, -16]!
    stp x8, x9, [sp, -16]!
    stp x10, x11, [sp, -16]!
    stp x12, x13, [sp, -16]!
    stp x14, x15, [sp, -16]!
    stp x16, x17, [sp, -16]!
    stp x18, x29, [sp, -16]!
    str x0, [sp, -16]!

    mov x0, 0       // Current thread (None)
    mrs x1, ESR_EL1 // Syndrome register
    mov x6, sp      // Where to put the return value, if any
    mov x7, 1       // Exception level
    ldr x9, =aarch64_handle_synchronous_exception
    blr x9

    ldr x0, [sp], 16
    ldp x18, x29, [sp], 16
    ldp x16, x17, [sp], 16
    ldp x14, x15, [sp], 16
    ldp x12, x13, [sp], 16
    ldp x10, x11, [sp], 16
    ldp x8, x9, [sp], 16
    ldp x6, x7, [sp], 16
    ldp x4, x5, [sp], 16
    ldp x2, x3, [sp], 16
    ldp x30, x1, [sp], 16
    eret
.Ls1xend:
.if .Ls1xend - synchronous_el1_spx > 0x80
    .err
.endif

.balign 0x80
irq_el1_spx:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    pushtemp

    ldr x9, =aarch64_handle_irq
    blr x9

    poptemp
    eret
.Li1xend:
.if .Li1xend - irq_el1_spx > 0x80
    .err
.endif

.balign 0x80
fiq_el1_spx:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    pushtemp

    ldr x9, =aarch64_handle_irq
    blr x9

    poptemp
    eret
.Lf1xend:
.if .Lf1xend - fiq_el1_spx > 0x80
    .err
.endif

.balign 0x80
serror_el1_spx:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    pushtemp

    // ****** TODO
    mov x0, '&'
    ldr x9, =putb
    blr x9
    // ******

    poptemp
    eret
.Le1xend:
.if .Le1xend - serror_el1_spx > 0x80
    .err
.endif

.balign 0x80
synchronous_el0_aarch64:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    stp x1, x6, [sp, -16]!
    stp x0, x7, [sp, -16]!
    ldr x0, [sp, 32]  // Current thread (pushed to the stack in scheduler.S)
    mrs x1, ESR_EL1   // Syndrome register
    mov x6, sp        // Where to put the return value, if any
    mov x7, 0         // Exception level
    bl aarch64_handle_synchronous_exception_indirect
    mov x30, x0
    ldp x0, x7, [sp], 16 // X0 = return value
    ldp x1, x6, [sp], 16

    mov sp, x19
    tst x30, 0xff00 // 0x00nn = ERET, 0x01nn = leave userspace
    b.eq .Ls06eret
    and x19, x30, 0x00ff
    ldr x20, =leave_userspace
    br x20
.Ls06eret:
    msr TTBR0_EL1, x20

    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Ls06end:
.if .Ls06end - synchronous_el0_aarch64 > 0x80
    .err
.endif

.balign 0x80
irq_el0_aarch64:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    str x0, [sp, -16]!
    bl aarch64_handle_irq_indirect
    cmp x0, 0 // 0 = normal ERET, 1 = leave userspace
	ldr x0, [sp], 16

    mov sp, x19
    b.eq .Li06eret
    mov x19, 0 // ThreadStatus::Running
    ldr x20, =leave_userspace
    br x20
.Li06eret:
    msr TTBR0_EL1, x20

    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Li06end:
.if .Li06end - irq_el0_aarch64 > 0x80
    .err
.endif

.balign 0x80
fiq_el0_aarch64:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    str x0, [sp, -16]!
    bl aarch64_handle_irq_indirect
    cmp x0, 0 // 0 = normal ERET, 1 = leave userspace
    ldr x0, [sp], 16

    mov sp, x19
    b.eq .Lf06eret
    mov x19, 0 // ThreadStatus::Running
    ldr x20, =leave_userspace
    br x20
.Lf06eret:
    msr TTBR0_EL1, x20

    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Lf06end:
.if .Lf06end - fiq_el0_aarch64 > 0x80
    .err
.endif

.balign 0x80
serror_el0_aarch64:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    // ****** TODO
    str x9, [sp, -16]!
    mov x9, '_'
    bl print_char
    ldr x9, [sp], 16
    // ******

    msr TTBR0_EL1, x20

    mov sp, x19
    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Le06end:
.if .Le06end - serror_el0_aarch64 > 0x80
    .err
.endif

.balign 0x80
synchronous_el0_aarch32:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    // ****** TODO
    str x9, [sp, -16]!
    mov x9, '+'
    bl print_char
    ldr x9, [sp], 16
    // ******

    msr TTBR0_EL1, x20

    mov sp, x19
    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Ls03end:
.if .Ls03end - synchronous_el0_aarch32 > 0x80
    .err
.endif

.balign 0x80
irq_el0_aarch32:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    str x0, [sp, -16]!
    bl aarch64_handle_irq_indirect
    cmp x0, 0 // 0 = normal ERET, 1 = leave userspace
    ldr x0, [sp], 16

    mov sp, x19
    b.eq .Li03eret
    mov x19, 0 // ThreadStatus::Running
    ldr x20, =leave_userspace
    br x20
.Li03eret:
    msr TTBR0_EL1, x20

    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Li03end:
.if .Li03end - irq_el0_aarch32 > 0x80
    .err
.endif

.balign 0x80
fiq_el0_aarch32:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    str x0, [sp, -16]!
    bl aarch64_handle_irq_indirect
    cmp x0, 0 // 0 = normal ERET, 1 = leave userspace
    ldr x0, [sp], 16

    mov sp, x19
    b.eq .Lf03eret
    mov x19, 0 // ThreadStatus::Running
    ldr x20, =leave_userspace
    br x20
.Lf03eret:
    msr TTBR0_EL1, x20

    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Lf03end:
.if .Lf03end - fiq_el0_aarch32 > 0x80
    .err
.endif

.balign 0x80
serror_el0_aarch32:
.ifdef _armv8_2_
    msr UAO, 1
.endif
    str x30, [sp, -16]!
    stp x19, x20, [sp, -16]!

    // PERF: When possible, handle the exception entirely within the trampoline. But make sure
    // nothing sensitive is mapped in the trampoline, including its stack, to prevent Meltdown.
    mrs x20, TTBR0_EL1
    bl use_kernel_root_page_table

    // Before pushing anything else, we should switch to the private kernel stack. It's bigger and
    // protected from Meltdown.
    ldr x19, [sp, 32] // The kernel's private stack pointer was saved on the stack in scheduler.S.
    mov x30, sp
    mov sp, x19
    mov x19, x30 // X19 = trampoline stack pointer

    // ****** TODO
    str x9, [sp, -16]!
    mov x9, '~'
    bl print_char
    ldr x9, [sp], 16
    // ******

    msr TTBR0_EL1, x20

    mov sp, x19
    ldp x19, x20, [sp], 16
    ldr x30, [sp], 16
    eret
.Le03end:
.if .Le03end - serror_el0_aarch32 > 0x80
    .err
.endif

// These procedures are down here to reduce the size of some of the exception handlers. They were
// starting to overflow their allotted space.
use_kernel_root_page_table:
    stp x9, x10, [sp, -16]!

    ldr x9, =ROOT_PAGE_TABLE_ADDR_TRAMP
    ldr x9, [x9]

    // Bits 48-51 of the page table address, if they're used, need to be moved to bits 2-5.
    and x10, x9, 0x000f000000000000
    and x9, x9,  0x0000ffffffffffc0
    orr x9, x9, x10, lsr (48 - 2)

    msr TTBR0_EL1, x9

    ldp x9, x10, [sp], 16
    ret

aarch64_handle_synchronous_exception_indirect:
    stp x1, x2, [sp, -16]!
    stp x3, x4, [sp, -16]!
    stp x5, x6, [sp, -16]!
    stp x7, x8, [sp, -16]!
    stp x9, x10, [sp, -16]!
    stp x11, x12, [sp, -16]!
    stp x13, x14, [sp, -16]!
    stp x15, x16, [sp, -16]!
    stp x17, x18, [sp, -16]!
    stp x29, x30, [sp, -16]!

    ldr x9, =aarch64_handle_synchronous_exception
    blr x9

    ldp x29, x30, [sp], 16
    ldp x17, x18, [sp], 16
    ldp x15, x16, [sp], 16
    ldp x13, x14, [sp], 16
    ldp x11, x12, [sp], 16
    ldp x9, x10, [sp], 16
    ldp x7, x8, [sp], 16
    ldp x5, x6, [sp], 16
    ldp x3, x4, [sp], 16
    ldp x1, x2, [sp], 16
    ret

aarch64_handle_irq_indirect:
    stp x1, x2, [sp, -16]!
    stp x3, x4, [sp, -16]!
    stp x5, x6, [sp, -16]!
    stp x7, x8, [sp, -16]!
    stp x9, x10, [sp, -16]!
    stp x11, x12, [sp, -16]!
    stp x13, x14, [sp, -16]!
    stp x15, x16, [sp, -16]!
    stp x17, x18, [sp, -16]!
    stp x29, x30, [sp, -16]!
    
    ldr x9, =aarch64_handle_irq
    blr x9

    ldp x29, x30, [sp], 16
    ldp x17, x18, [sp], 16
    ldp x15, x16, [sp], 16
    ldp x13, x14, [sp], 16
    ldp x11, x12, [sp], 16
    ldp x9, x10, [sp], 16
    ldp x7, x8, [sp], 16
    ldp x5, x6, [sp], 16
    ldp x3, x4, [sp], 16
    ldp x1, x2, [sp], 16
    ret


// TODO: This procedure serves no purpose other than to mark an exception as unimplemented. Remove
// it when it's no longer used.
print_char:
    pushtemp

    mov x0, x9
    ldr x9, =putb
    blr x9

    poptemp
    ret
