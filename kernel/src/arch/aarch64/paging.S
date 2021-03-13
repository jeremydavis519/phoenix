/* Copyright (c) 2018-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

.global __start, __end

.global init_paging
.global boot_panic

.global ROOT_PAGE_TABLE_ADDR

// These constants are also defined in /crates/memory/src/virt/paging/aarch64.rs.
// Make sure to keep them up to date.
.set ATTR_NORMAL_MEMORY, 0
//.set ATTR_DEVICE_MEMORY, 1

.section .text.boot.paging

// Enables paging. A translation table will be set up at KERNEL_TRANSLATION_TABLE that identity-maps the entire physical memory space.
// Additionally, this address space will also be mapped to the higher half of virtual memory. The higher-half mapping should be used
// only by trampoline code that switches the lower-half address space between kernelspace and userspace, since running the whole kernel
// in the higher half would leave it vulnerable to Spectre and Meltdown.
init_paging:
    str lr, [sp, -16]!

    // Set up the caching hints for different kinds of memory and MMIO.
    mov x9, 0xff  // ATTR_NORMAL_MEMORY: Normal memory, outer write-back non-transient, inner write-back non-transient, allocation hints
.if ATTR_NORMAL_MEMORY // No need to shift if we're shifting by 0 bits.
    lsl x9, x9, ATTR_NORMAL_MEMORY * 8
.endif
                   // ATTR_DEVICE_MEMORY: Device-nGnRnE (non-Gathering, non-Reordering, no Early write acknowledgement; i.e. strongly ordered)
                   // (The byte that specifies this set of hints is 0, so no additional instructions are needed.)
    msr MAIR_EL1, x9

    // Set up everything else needed for paging.
    mrs x10, TCR_EL1 // X10 = TCR_EL1, with the necessary modifications
    and x10, x10, 0xfffffff83fffffff // Holes for TCR_EL1.IPS and TCR_EL1.TG1
    and x10, x10, 0xffffffffffc03fff // Hole for TCR_EL1.T1SZ and TCR_EL1.TG0
    and x10, x10, 0xffffffffffffffc0 // Hole for TCR_EL1.T0SZ

    // Allow all physical address bits up to a max of 52 by setting TCR_EL1.IPS to min(ID_AA64MMFR0_EL1.PARANGE, 0b110).
    mrs x9, ID_AA64MMFR0_EL1
    and x9, x9, 0xf
    cmp x9, 0b0110
    b.le .Lset_ips
    mov x9, 0b0110
.Lset_ips:
    ldr x2, =PAGE_SIZES
    ldrb w2, [x2, x9] // W2 = physical addressable bits
    lsl x9, x9, 32
    orr x10, x10, x9

    // Set TCR_EL1.T0SZ and TCR_EL1.T1SZ to (64 - x) (i.e. allow x virtual address bits),
    //  where x = 48 if ID_AA64MMFR2_EL1.VARange = 0
    //            52 if ID_AA64MMFR2_EL1.VARange = 1 (i.e. the system supports ARMv8.2-LVA)
    //            48 if ID_AA64MMFR2_EL1.VARange is any reserved value.
    mov x1, 48 // X1 = virtual addressable bits
    mov x11, (64 - 48) // X11 = 64 - x
.ifdef _armv8_2_
    mrs x9, ID_AA64MMFR2_EL1
    and x9, x9, 0xf0000
    cmp x9, 0x10000
    b.ne .Lva_size_done
    mov x1, 52 // X1 = addressable bits
    mov x11, (64 - 52)
.Lva_size_done:
.endif
    orr x10, x10, x11          // TCR_EL1.T0SZ
    orr x10, x10, x11, lsl 16 // TCR_EL1.T1SZ

    // By default, set the page size to the highest one supported to reduce time spent allocating pages and translating virtual addresses.
    // TODO: Different priorities might be given to the page sizes for different target architectures. For instance, one that places MMIO
    //       regions or physical memory regions at 4-kiB boundaries should probably use 4-KiB pages even if 64-KiB or 16-KiB pages are supported.

    // TCR_EL1.TG0 (which determines page size) becomes
    //  0b01 (64 kiB) if ID_AA64MMFR0_EL1.TGRAN64 = 0b0000, else
    //  0b10 (16 kiB) if ID_AA64MMFR0_EL1.TGRAN16 = 0b0001, else (note: this one's different)
    //  0b00 (4 kiB) if ID_AA64MMFR0_EL1.TGRAN4 = 0b0000, else
    // TCR_EL1.TG1 (determining page size in the trampoline) expects different values for some reason:
    //  0b11 for 64 kiB
    //  0b01 for 16 kiB
    //  0b10 for 4 kiB
    mrs x9, ID_AA64MMFR0_EL1
    tst x9, 0x0f000000
    b.eq .L64k
    and x11, x9, 0x00f00000
    cmp x11, 0x00100000
    b.eq .L16k
    tst x9, 0xf0000000
    b.eq .L4k
    // panic: no supported page size
    ldr x0, =PANIC_NO_PAGE_SIZE
    b boot_panic
.L64k:
    orr x10, x10, 0x4000     // TG0
    orr x10, x10, 0xc0000000 // TG1
    mov x0, 0x10000          // X0 = page size
    b .Lpage_size_done
.L16k:
    orr x10, x10, 0x8000
    orr x10, x10, 0x40000000
    mov x0, 0x4000
    b .Lpage_size_done
.L4k: // TG0 is already set to 0.
    orr x10, x10, 0x80000000
    mov x0, 0x1000
.Lpage_size_done:

    // If the hardware supports it, turn on automatic handling of the Accessed and Dirty flags.
.ifdef _armv8_1_
    mrs x9, IDAA64MMFR1_EL1
    and x9, x9, 0xf // X9 = IDAA64MMFR1_EL1.HAFDBS
    cmp x9, 0x1
    b.eq .Laccess_only
    cmp x9, 0x2
    b.ne .Lno_hw_afdb_support // 0x0 means unsupported, and all other values are reserved.
    orr x10, x10, 0x0000010000000000 // TCR_EL1.HD
.Laccess_only:
    orr x10, x10, 0x0000008000000000 // TCR_EL1.HA
.Lno_hw_afdb_support:
.endif

    msr TCR_EL1, x10

.ifdef _armv8_1_
    // Set PSTATE.PAN so that EL1 code can't directly access virtual memory owned by an EL0 process.
    // This will prevent any possibility of an exploit involving running a userspace process's code
    // with the kernel's privileges.
    msr PAN, 1
.endif

.ifdef _armv8_2_
    // PSTATE.UAO hardens the divide between EL0 and EL1 even more by preventing EL1 from effectively
    // lowering its privilege level in order to read or write some data as if it were EL0. We should
    // never need to do that, so we might as well remove the possibility.
    msr UAO, 1
.endif

    // Initialize the translation tables.
    str x1, [sp, -16]!
    bl init_page_tables
    // X0 = address of the root table
    ldr x1, =ROOT_PAGE_TABLE_ADDR
    str x0, [x1]
    ldr x1, [sp], 16

    // Tell TTBR0_EL1 where the root translation table is.

    // Bits 48-51 of the address, if they're used, need to be moved to bits 2-5.
    and x9, x0, 0x000f000000000000
    and x0, x0, 0x0000ffffffffffc0
    orr x0, x0, x9, lsr (48 - 2)

    // If "Copy not Private" is supported, set that bit so all PEs will be able to share the kernelspace TLB entries.
.ifdef _armv8_2_
    mrs x11, ID_AA64MMFR2_EL1
    and x11, x11, 0xf
    cmp x11, 0b0001
    b.ne .Lno_cnp
    orr x0, x0, 0b1
.Lno_cnp:
.endif

    // We'll leave TTBR0_EL1.ASID as 0 to represent the kernel's address space.

    msr TTBR0_EL1, x0

    // PERF: We should have a boot option that keeps the entire kernel mapped in the upper address
    // space (like Linux's option to disable KPTI), since not all CPUs are susceptible to Meltdown.
    // All of the kernel's code that normally runs in the lower address space will have to be made
    // relocatable or position-independent to make this work, and physical addresses will need to
    // be changed when converted to virtual addresses, even if we keep "identity-mapping" them.

    // Initialize the translation table for the upper half of memory.
    mov x0, x1
    bl init_trampoline_page_tables
    // X0 = translation table address

    // Tell TTBR1_EL1 where the root translation table is.

    // Bits 48-51 of the address, if they're used, need to be moved to bits 2-5.
    and x9, x0, 0x000f000000000000
    and x0, x0, 0x0000ffffffffffc0
    orr x0, x0, x9, lsr (48 - 2)

    // If "Copy not Private" is supported, set that bit so all PEs will be able to share the kernelspace TLB entries.
.ifdef _armv8_2_
    mrs x11, ID_AA64MMFR2_EL1
    and x11, x11, 0xf
    cmp x11, 0b0001
    b.ne .Lno_cnp
    orr x0, x0, 0b1
.Lno_cnp:
.endif

    // We'll leave TTBR1_EL1.ASID as 0 to represent the kernel's address space.

    msr TTBR1_EL1, x0

    // Enable address translation by setting SCTLR_EL1.M. Also set the WXN (writable execute never) bit so writable pages
    // will never be executable, regardless of what their descriptors say and the C bit so memory is cacheable.
    mrs x9, SCTLR_EL1
    orr x9, x9, 0x1     // M (MMU enabled)
    orr x9, x9, 0x4     // C (Normal memory can be cached)
    orr x9, x9, 0x80000 // WXN (Write Execute Never)
    msr SCTLR_EL1, x9

    // Make sure the instruction fetcher uses the new MMU settings. (This shouldn't matter as long as we're identity-mapping,
    // but it's a good practice in general.)
    isb

    ldr lr, [sp], 16
    ret


// The trampoline data sections are read-only at their natural addresses but read-write at their trampoline addresses.
.section .trampoline.data.paging, "a", @progbits
ROOT_PAGE_TABLE_ADDR: .8byte 0


.section .rodata.paging, "a", @progbits
PANIC_NO_PAGE_SIZE: .asciz "no supported page size found"

PAGE_SIZES: .byte 32, 36, 40, 42, 44, 48, 52
