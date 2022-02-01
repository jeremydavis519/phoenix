/* Copyright (c) 2019-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

// Pushes the registers that the callee is required to preserve.
.macro pushcallee
    stp x19, x20, [sp, -16]!
    stp x21, x22, [sp, -16]!
    stp x23, x24, [sp, -16]!
    stp x25, x26, [sp, -16]!
    stp x27, x28, [sp, -16]!
.endm

// Pops the registers that the callee is required to preserve.
.macro popcallee
    ldp x27, x28, [sp], 16
    ldp x25, x26, [sp], 16
    ldp x23, x24, [sp], 16
    ldp x21, x22, [sp], 16
    ldp x19, x20, [sp], 16
.endm

// Pushes the registers that the callee is not required to preserve.
.macro pushtemp
    stp x0, x1, [sp, -16]!
    stp x2, x3, [sp, -16]!
    stp x4, x5, [sp, -16]!
    stp x6, x7, [sp, -16]!
    stp x8, x9, [sp, -16]!
    stp x10, x11, [sp, -16]!
    stp x12, x13, [sp, -16]!
    stp x14, x15, [sp, -16]!
    stp x16, x17, [sp, -16]!
    stp x18, x29, [sp, -16]!
    str x30, [sp, -16]!
.endm

// Pops the registers that the callee is not required to preserve.
.macro poptemp
    ldr x30, [sp], 16
    ldp x18, x29, [sp], 16
    ldp x16, x17, [sp], 16
    ldp x14, x15, [sp], 16
    ldp x12, x13, [sp], 16
    ldp x10, x11, [sp], 16
    ldp x8, x9, [sp], 16
    ldp x6, x7, [sp], 16
    ldp x4, x5, [sp], 16
    ldp x2, x3, [sp], 16
    ldp x0, x1, [sp], 16
.endm

// Pushes the floating-point registers.
.macro pushfp
    stp q0, q1, [sp, -32]!
    stp q2, q3, [sp, -32]!
    stp q4, q5, [sp, -32]!
    stp q6, q7, [sp, -32]!
    stp q8, q9, [sp, -32]!
    stp q10, q11, [sp, -32]!
    stp q12, q13, [sp, -32]!
    stp q14, q15, [sp, -32]!
    stp q16, q17, [sp, -32]!
    stp q18, q19, [sp, -32]!
    stp q20, q21, [sp, -32]!
    stp q22, q23, [sp, -32]!
    stp q24, q25, [sp, -32]!
    stp q26, q27, [sp, -32]!
    stp q28, q29, [sp, -32]!
    stp q30, q31, [sp, -32]!
.endm

// Pops the floating-point registers.
.macro popfp
    ldp q30, q31, [sp], 32
    ldp q28, q29, [sp], 32
    ldp q26, q27, [sp], 32
    ldp q24, q25, [sp], 32
    ldp q22, q23, [sp], 32
    ldp q20, q21, [sp], 32
    ldp q18, q19, [sp], 32
    ldp q16, q17, [sp], 32
    ldp q14, q15, [sp], 32
    ldp q12, q13, [sp], 32
    ldp q10, q11, [sp], 32
    ldp q8, q9, [sp], 32
    ldp q6, q7, [sp], 32
    ldp q4, q5, [sp], 32
    ldp q2, q3, [sp], 32
    ldp q0, q1, [sp], 32
.endm
