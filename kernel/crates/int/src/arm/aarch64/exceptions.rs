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

use {
    core::{
        arch::asm,
        convert::{TryFrom, TryInto},
        num::NonZeroUsize,
    },
    bitflags::bitflags,
    volatile::Volatile,
    fs::File,
    io::printlndebug,
    macros_unreachable::unreachable_debug,
    memory::virt::paging,
    paging::{ExceptionLevel, PageStatus, TranslationLevel},
    scheduler::{Thread, ThreadStatus},

    super::syscall::handle_system_call,
};

bitflags! {
    #[repr(transparent)]
    struct Syndrome: u32 {
        const EXCEPTION_CLASS = 0xfc000000;
        const NOT_16BIT       = 0x02000000;
        const INSTR_SPECIFIC  = 0x01ffffff;

        const EC_UNKNOWN                    = 0x00000000;
        const EC_TRAP_WFI_WFE               = 0x04000000;
        const EC_TRAP_MCR_MRC_COPROC_1111   = 0x0c000000;
        const EC_TRAP_MCRR_MRRC_COPROC_1111 = 0x10000000;
        const EC_TRAP_MCR_MRC_COPROC_1110   = 0x14000000;
        const EC_TRAP_LDC_STC               = 0x18000000;
        const EC_TRAP_SVE_SIMD_FP           = 0x1c000000;
        const EC_TRAP_MRRC_COPROC_1110      = 0x30000000;
        const EC_ILLEGAL_EXECUTION_STATE    = 0x38000000;
        const EC_SVC_AARCH32                = 0x44000000;
        const EC_SVC_AARCH64                = 0x54000000;
        const EC_TRAP_SYS_INSTR_AARCH64     = 0x60000000;
        const EC_TRAP_SVE                   = 0x64000000;
        const EC_INSTR_ABORT_EL0            = 0x80000000;
        const EC_INSTR_ABORT_EL1            = 0x84000000;
        const EC_PC_ALIGNMENT               = 0x88000000;
        const EC_DATA_ABORT_EL0             = 0x90000000;
        const EC_DATA_ABORT_EL1             = 0x94000000;
        const EC_SP_ALIGNMENT               = 0x98000000;
        const EC_TRAP_FP_EXCEPTION_AARCH32  = 0xa0000000;
        const EC_TRAP_FP_EXCEPTION_AARCH64  = 0xb0000000;
        const EC_SERROR                     = 0xbc000000;
        const EC_BREAKPOINT_EL0             = 0xc0000000;
        const EC_BREAKPOINT_EL1             = 0xc4000000;
        const EC_SINGLE_STEP_EL0            = 0xc8000000;
        const EC_SINGLE_STEP_EL1            = 0xcc000000;
        const EC_WATCHPOINT_EL0             = 0xd0000000;
        const EC_WATCHPOINT_EL1             = 0xd4000000;
        const EC_BKPT_INSTR_AARCH32         = 0xe0000000;
        const EC_BRK_INSTR_AARCH64          = 0xf0000000;
    }
}

bitflags! {
    struct MmuIss: u32 {
        const FAULT_STATUS_CODE      = 0x0000003f;
        const CAUSED_BY_WRITE        = 0x00000040;
        const CACHE_MAINTENANCE      = 0x00000100;
        const EXTERNAL_ABORT_TYPE    = 0x00000200; // Implementation-defined
        const FAR_NOT_VALID          = 0x00000400;
        const SYNC_ERROR_TYPE        = 0x00001800;
        const ACQUIRE_RELEASE        = 0x00004000;
        const REGISTER_64BIT         = 0x00008000;
        const REGISTER_NUMBER        = 0x001f0000;
        const SIGN_EXTENDED          = 0x00200000;
        const ACCESS_SIZE            = 0x00c00000;
        const INSTR_SYNDROME_VALID   = 0x01000000; // If 0, then FAR_NOT_VALID to ACCESS_SIZE are all RES0.

        const SET_RECOVERABLE      = 0x00000000;
        const SET_RESTARTABLE      = 0x00000800;
        const SET_UNCONTAINABLE    = 0x00001000;
        const SET_CORRECTED        = 0x00001800;

        const FS_ADDR_SIZE_LEVEL_0      = 0x00000000;
        const FS_ADDR_SIZE_LEVEL_1      = 0x00000001;
        const FS_ADDR_SIZE_LEVEL_2      = 0x00000002;
        const FS_ADDR_SIZE_LEVEL_3      = 0x00000003;
        const FS_TRANSLATION_LEVEL_0    = 0x00000004;
        const FS_TRANSLATION_LEVEL_1    = 0x00000005;
        const FS_TRANSLATION_LEVEL_2    = 0x00000006;
        const FS_TRANSLATION_LEVEL_3    = 0x00000007;
        const FS_ACCESS_FLAG_LEVEL_1    = 0x00000009;
        const FS_ACCESS_FLAG_LEVEL_2    = 0x0000000a;
        const FS_ACCESS_FLAG_LEVEL_3    = 0x0000000b;
        const FS_PERMISSION_LEVEL_1     = 0x0000000d;
        const FS_PERMISSION_LEVEL_2     = 0x0000000e;
        const FS_PERMISSION_LEVEL_3     = 0x0000000f;
        const FS_SYNC_EXTERNAL          = 0x00000010; // Not on a translation table walk
        const FS_SYNC_EXTERNAL_LEVEL_0  = 0x00000014;
        const FS_SYNC_EXTERNAL_LEVEL_1  = 0x00000015;
        const FS_SYNC_EXTERNAL_LEVEL_2  = 0x00000016;
        const FS_SYNC_EXTERNAL_LEVEL_3  = 0x00000017;
        const FS_SYNC_PARITY            = 0x00000018; // Not on a translation table walk
        const FS_SYNC_PARITY_LEVEL_0    = 0x0000001c;
        const FS_SYNC_PARITY_LEVEL_1    = 0x0000001d;
        const FS_SYNC_PARITY_LEVEL_2    = 0x0000001e;
        const FS_SYNC_PARITY_LEVEL_3    = 0x0000001f;
        const FS_ALIGNMENT              = 0x00000021;
        const FS_TLB_CONFLICT           = 0x00000030;
        const FS_ATOMIC_HARDWARE_UPDATE = 0x00000031;
        const FS_SECTION_DOMAIN         = 0x0000003d;
        const FS_PAGE_DOMAIN            = 0x0000003e;
    }
}

bitflags! {
    struct SvcIss: u32 {
        const SERVICE = 0x0000ffff;
    }
}

impl MmuIss {
    fn access_size(&self) -> NonZeroUsize {
        NonZeroUsize::new(
            if self.contains(MmuIss::INSTR_SYNDROME_VALID) {
                1 << ((*self & MmuIss::ACCESS_SIZE).bits() >> MmuIss::ACCESS_SIZE.bits().trailing_zeros())
            } else {
                1
            }
        ).unwrap()
    }
}

// Represents the action that needs to be taken to properly return from the exception handler. This
// is returned by the Rust exception handlers and interpreted by the ASM exception handlers.
#[cfg(target_endian = "little")]
#[repr(C)]
#[must_use]
pub(crate) struct Response {
    status: ThreadStatus,
    action: ExitAction,
}

#[cfg(target_endian = "big")]
#[repr(C)]
#[must_use]
pub(crate) struct Response {
    action: ExitAction,
    status: ThreadStatus,
}

impl Response {
    pub(crate) fn eret() -> Response {
        Response { action: ExitAction::Eret, status: ThreadStatus::Running }
    }

    pub(crate) fn leave_userspace(status: ThreadStatus) -> Response {
        Response { action: ExitAction::LeaveUserspace, status }
    }

    /*pub(crate) fn retry_syscall() -> Response {
        Response { action: ExitAction::RetrySyscall, status: ThreadStatus::Running }
    }*/
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum ExitAction {
    // Execute an ERET instruction as would normally be done.
    Eret = 0,
    // Leave userspace and execute a context switch. This action is meaningless if the exception
    // wasn't taken from userspace, so returning it is undefined behavior in that case.
    LeaveUserspace = 1,
    // Retry the system call because it couldn't be finished in constant time. This allows the
    // scheduler to pre-empt the thread if the timer has expired.
    #[allow(dead_code)] // TODO: Remove this marker when this variant is actually used.
    RetrySyscall = 2,
    // Leave userspace as with `LeaveUserspace`, but retry the system call immediately afterward.
    // This shouldn't be returned directly by system calls; it's an implementation detail for when
    // `RetrySyscall` results in a thread being pre-empted.
    LeaveUserspaceAndRetrySyscall = 3,
}

// TODO: Make all of these references to `Thread`s generic somehow.

#[no_mangle]
extern fn aarch64_handle_synchronous_exception(
    thread: Option<&mut Thread<File>>,
    syndrome: Syndrome,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    result: *mut [usize; 2],
    exc_level: u8,
) -> Response {
    assert!(!result.is_null());
    let result = Volatile::new_write_only(unsafe { &mut *result });
    let args = [arg1, arg2, arg3, arg4];
    let exc_level = ExceptionLevel::try_from(exc_level).expect("unrecognized exception level");

    // TODO: Finish implementing all of these.
    let mut response = match syndrome & Syndrome::EXCEPTION_CLASS {
        Syndrome::EC_UNKNOWN                    =>
            panic!("unknown exception occurred at address {:#x}", elr()),
        Syndrome::EC_TRAP_WFI_WFE               => unimplemented!(),
        Syndrome::EC_TRAP_MCR_MRC_COPROC_1111   => unimplemented!(),
        Syndrome::EC_TRAP_MCRR_MRRC_COPROC_1111 => unimplemented!(),
        Syndrome::EC_TRAP_MCR_MRC_COPROC_1110   => unimplemented!(),
        Syndrome::EC_TRAP_LDC_STC               => unimplemented!(),
        Syndrome::EC_TRAP_SVE_SIMD_FP           => unimplemented!(),
        Syndrome::EC_TRAP_MRRC_COPROC_1110      => unimplemented!(),
        Syndrome::EC_ILLEGAL_EXECUTION_STATE    => unimplemented!(),
        Syndrome::EC_SVC_AARCH32                => {
            handle_system_call(thread, SvcIss::from_bits_truncate(syndrome.bits()).bits().try_into().unwrap(), &args, result)
        },
        Syndrome::EC_SVC_AARCH64                => {
            handle_system_call(thread, SvcIss::from_bits_truncate(syndrome.bits()).bits().try_into().unwrap(), &args, result)
        },
        Syndrome::EC_TRAP_SYS_INSTR_AARCH64     => unimplemented!(),
        Syndrome::EC_TRAP_SVE                   => unimplemented!(),
        Syndrome::EC_INSTR_ABORT_EL0            => handle_mmu_abort(thread.map(|t| &*t), syndrome, exc_level),
        Syndrome::EC_INSTR_ABORT_EL1            => handle_mmu_abort(thread.map(|t| &*t), syndrome, exc_level),
        Syndrome::EC_PC_ALIGNMENT               => unimplemented!(),
        Syndrome::EC_DATA_ABORT_EL0             => handle_mmu_abort(thread.map(|t| &*t), syndrome, exc_level),
        Syndrome::EC_DATA_ABORT_EL1             => handle_mmu_abort(thread.map(|t| &*t), syndrome, exc_level),
        Syndrome::EC_SP_ALIGNMENT               => unimplemented!(),
        Syndrome::EC_TRAP_FP_EXCEPTION_AARCH32  => unimplemented!(),
        Syndrome::EC_TRAP_FP_EXCEPTION_AARCH64  => unimplemented!(),
        Syndrome::EC_SERROR                     => unimplemented!(),
        Syndrome::EC_BREAKPOINT_EL0             => unimplemented!(),
        Syndrome::EC_BREAKPOINT_EL1             => unimplemented!(),
        Syndrome::EC_SINGLE_STEP_EL0            => unimplemented!(),
        Syndrome::EC_SINGLE_STEP_EL1            => unimplemented!(),
        Syndrome::EC_WATCHPOINT_EL0             => unimplemented!(),
        Syndrome::EC_WATCHPOINT_EL1             => unimplemented!(),
        Syndrome::EC_BKPT_INSTR_AARCH32         => unimplemented!(),
        Syndrome::EC_BRK_INSTR_AARCH64          => unimplemented!(),
        _ => panic!("unrecognized exception syndrome (ESR_EL1) {:#x}", syndrome.bits())
    };

    if let ExceptionLevel::El1 = exc_level {
        // Trying a context switch from the kernel is undefined behavior.
        assert_ne!(response.action, ExitAction::LeaveUserspace);
    }

    match response.action {
        ExitAction::Eret => {
            // If the scheduling timer went off while we were handling the exception, we should still
            // leave userspace.
            if timers::scheduling_timer_finished() {
                printlndebug!("Scheduling timer finished during synchronous exception");
                response.action = ExitAction::LeaveUserspace;
            }
            response
        },
        ExitAction::RetrySyscall => {
            // If the scheduling timer went off during a syscall that couldn't be finished, we
            // should leave userspace after preparing to retry the syscall.
            if timers::scheduling_timer_finished() {
                printlndebug!("Scheduling timer finished during unfinished syscall");
                response.action = ExitAction::LeaveUserspaceAndRetrySyscall;
            }
            response
        },
        ExitAction::LeaveUserspace | ExitAction::LeaveUserspaceAndRetrySyscall => response,
    }
}

fn handle_mmu_abort(thread: Option<&Thread<File>>, syndrome: Syndrome, exc_level: ExceptionLevel) -> Response {
    let iss = MmuIss::from_bits_truncate(syndrome.bits());

    // TODO: Finish implementing all of these.
    match iss & MmuIss::FAULT_STATUS_CODE {
        MmuIss::FS_ADDR_SIZE_LEVEL_0      => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_ADDR_SIZE_LEVEL_1      => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_ADDR_SIZE_LEVEL_2      => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_ADDR_SIZE_LEVEL_3      => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        // TODO: In each of these 4 cases, map the appropriate page if it can be found.
        //       For EL0, look in `thread`'s executable file and (when implemented) its swap file.
        //       For EL1, we can use this opportunity to identity-map a new read-write page, which should reduce the size of the
        //          kernel's page tables and speed up the boot process.
        MmuIss::FS_TRANSLATION_LEVEL_0    => try_map_page(thread, fault_address(), iss),
        MmuIss::FS_TRANSLATION_LEVEL_1    => try_map_page(thread, fault_address(), iss),
        MmuIss::FS_TRANSLATION_LEVEL_2    => try_map_page(thread, fault_address(), iss),
        MmuIss::FS_TRANSLATION_LEVEL_3    => try_map_page(thread, fault_address(), iss),
        MmuIss::FS_ACCESS_FLAG_LEVEL_1    => {
            paging::set_accessed_flag(thread.map(|t| t.process.exec_image.page_table()), TranslationLevel::Level1, fault_address());
            Response::eret()
        },
        MmuIss::FS_ACCESS_FLAG_LEVEL_2    => {
            paging::set_accessed_flag(thread.map(|t| t.process.exec_image.page_table()), TranslationLevel::Level2, fault_address());
            Response::eret()
        },
        MmuIss::FS_ACCESS_FLAG_LEVEL_3    => {
            paging::set_accessed_flag(thread.map(|t| t.process.exec_image.page_table()), TranslationLevel::Level3, fault_address());
            Response::eret()
        },
        MmuIss::FS_PERMISSION_LEVEL_1     => handle_permission_fault(thread, exc_level, TranslationLevel::Level1, fault_address(), iss),
        MmuIss::FS_PERMISSION_LEVEL_2     => handle_permission_fault(thread, exc_level, TranslationLevel::Level2, fault_address(), iss),
        MmuIss::FS_PERMISSION_LEVEL_3     => handle_permission_fault(thread, exc_level, TranslationLevel::Level3, fault_address(), iss),
        MmuIss::FS_SYNC_EXTERNAL          => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_SYNC_EXTERNAL_LEVEL_0  => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_SYNC_EXTERNAL_LEVEL_1  => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_SYNC_EXTERNAL_LEVEL_2  => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_SYNC_EXTERNAL_LEVEL_3  => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_SYNC_PARITY            => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_SYNC_PARITY_LEVEL_0    => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_SYNC_PARITY_LEVEL_1    => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_SYNC_PARITY_LEVEL_2    => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_SYNC_PARITY_LEVEL_3    => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_ALIGNMENT              => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_TLB_CONFLICT           => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_ATOMIC_HARDWARE_UPDATE => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_SECTION_DOMAIN         => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        MmuIss::FS_PAGE_DOMAIN            => unimplemented!("syndrome = {:#x}", syndrome.bits()),
        _ => panic!("unrecognized instruction-specific syndrome (ESR_EL1.ISS) {:#x}", iss.bits())
    }
}

fn fault_address() -> usize {
    let address: usize;
    unsafe {
        asm!("mrs {}, FAR_EL1", out(reg) address, options(nomem, nostack, preserves_flags));
    }
    address
}

fn elr() -> u64 {
    let elr: u64;
    unsafe {
        asm!("mrs {}, ELR_EL1", out(reg) elr, options(nomem, nostack, preserves_flags));
    }
    elr
}

fn try_map_page(thread: Option<&Thread<File>>, fault_address: usize, iss: MmuIss) -> Response {
    if let Some(thread) = thread {
        let access_size = iss.access_size();

        // Try the swapfile first.
        // TODO: Make the swapfile logic aware of the access size somehow.
        match load_page_from_swapfile(thread, fault_address) {
            Ok(()) => return Response::eret(), // Successfully mapped
            Err(()) => {}
        };
        // The unmapped page isn't in the swapfile, so look in the thread's executable file.
        match thread.process.exec_image.load_segment_piece(fault_address, access_size) {
            Ok(Some(block)) => { // Successfully mapped
                // FIXME: Instead of forgetting the block, push it onto a vector of blocks owned by
                // the thread's process.
                core::mem::forget(block);
                return Response::eret();
            },
            Ok(None) => { // Mapped to an existing block (e.g. CoW)
                return Response::eret();
            },
            Err(None) => { // Not mapped, but we should try again later
                return Response::leave_userspace(ThreadStatus::Running)
            },
            Err(_) => {} // Failed
        };

        match thread.process.exec_image.page_table().page_status(fault_address) {
            // If the page is already mapped, that means another CPU has already resolved this fault.
            PageStatus::Mapped       => Response::eret(),

            // If it's only temporarily unmapped, another CPU is in the process of resolving it.
            PageStatus::TempUnmapped => Response::leave_userspace(ThreadStatus::Running),

            // If the page is still permanently unmapped, the thread has tried to access memory
            // that doesn't exist.
            PageStatus::Unmapped     => Response::leave_userspace(ThreadStatus::Terminated)
        }
    } else {
        // TODO: This is where we would implement the lazy mapping of the kernel's memory.
        unimplemented!();
    }
}

fn load_page_from_swapfile(thread: &Thread<File>, fault_address: usize) -> Result<(), ()> {
    let page_table = thread.process.exec_image.page_table();
    if let Some(_location) = page_table.location_in_swapfile(fault_address) {
        // TODO
        unimplemented!("load the page from the swapfile");
    } else {
        // The page isn't in the swapfile.
        Err(())
    }
}

fn handle_permission_fault(thread: Option<&Thread<File>>, exc_level: ExceptionLevel, trans_level: TranslationLevel, fault_address: usize, iss: MmuIss)
        -> Response {
    // The kernel doesn't use CoW internally, nor does it write to CoW pages in userspace, and all
    // of its writable pages are mapped pre-dirtied. Therefore, the kernel should never generate a
    // Permission Fault.
    let thread = thread.expect("Permission Fault caused by a kernel thread");

    // TODO: Instead of just printing `iss.bits()`, parse `iss` to produce a more readable error
    // message.

    match iss & MmuIss::SYNC_ERROR_TYPE {
        MmuIss::SET_RECOVERABLE => {}, // We'll try to recover from the error.
        MmuIss::SET_RESTARTABLE => panic!("restartable permission fault occurred accessing address {:#018x}: ISS = {:#010x}", fault_address, iss.bits()),
        MmuIss::SET_UNCONTAINABLE => panic!("uncontainable permission fault occurred accessing address {:#018x}: ISS = {:#010x}", fault_address, iss.bits()),
        MmuIss::SET_CORRECTED => return Response::eret(), // Nothing to do if it's already been corrected.
        _ => unsafe {
            unreachable_debug!("SYNC_ERROR_TYPE has only 2 bits, and all 4 possibilities have been checked.")
        }
    };

    if iss.contains(MmuIss::CAUSED_BY_WRITE) {
        match paging::resolve_write_fault(thread.process.exec_image.page_table(), exc_level, trans_level, fault_address, iss.access_size()) {
            Ok(block) => { // Resolved!
                if let Some(block) = block {
                    // FIXME: Instead of forgetting the block, push it onto a vector of blocks
                    // owned by the thread's process.
                    core::mem::forget(block);
                }
                return Response::eret();
            },
            Err(()) => {}
        };
    }

    // TODO: Send a signal or something instead of just directly printing this message (although we
    // might want to keep printing this from the kernel anyway).
    // TODO: Internationalize.
    let sp_el0: u64;
    unsafe {
        asm!("mrs {}, SP_EL0", out(reg) sp_el0, options(nomem, preserves_flags, nostack));
    }
    printlndebug!("killing thread: permission fault occurred accessing address {:#018x}: ISS = {:#010x}", fault_address, iss.bits());
    printlndebug!("  SP_EL0 = {:#010x}", sp_el0);
    Response::leave_userspace(ThreadStatus::Terminated)
}
