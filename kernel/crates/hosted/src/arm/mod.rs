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

//! ARM-specific implementations of the `hosted` API.

pub mod io;
pub mod fs;

#[cfg(target_pointer_width = "32")]
type Field = i32;
#[cfg(target_pointer_width = "64")]
type Field = i64;

bitflags! {
    struct Extensions: u8 {
        const EXIT_EXTENDED = 0x01;
        const STDOUT_STDERR = 0x02;
    }
}

// The list of semihosting extensions that the host supports.
lazy_static! {
    unsafe {
        static ref EXTENSIONS: Extensions = {
            // TODO
            unimplemented!();
        };
    }
}

#[repr(u32)]
enum Operation {
    // TODO: Support all of these operations.
    Open = 1,
    Close = 2,
    //WriteC = 3,
    //Write0 = 4,
    Write = 5,
    Read = 6,
    //ReadC = 7,
    //IsError = 8, (unsupported by QEMU)
    IsTty = 9,
    Seek = 10,
    FLen = 12,
    //TmpName = 13,
    Remove = 14,
    Rename = 15,
    //Clock = 16,
    //Time = 17,
    //System = 18,
    ErrNo = 19,
    //GetCmdLine = 21,
    //HeapInfo = 22,
    Exit = 24,
    //ExitExt = 32,
    //Elapsed = 48,
    //TickFreq = 49
}

#[cfg_attr(target_pointer_width = "32", repr(u32))]
#[cfg_attr(target_pointer_width = "64", repr(u64))]
enum ExitReason {
    // TODO: Support all of these exit reasons.
    //BranchThroughZero   = 0x20000,
    //UndefinedInstr      = 0x20001,
    //SoftwareInterrupt   = 0x20002,
    //PrefetchAbort       = 0x20003,
    //DataAbort           = 0x20004,
    //AddressException    = 0x20005,
    //UnhandledIrq        = 0x20006,
    //UnhandledFiq        = 0x20007,

    //Breakpoint          = 0x20020,
    //WatchPoint          = 0x20021,
    //StepComplete        = 0x20022,
    //RuntimeErrorUnknown = 0x20023,
    //InternalError       = 0x20024,
    //UserInterruption    = 0x20025,
    ApplicationExit     = 0x20026,
    //StackOverflow       = 0x20027,
    //DivisionByZero      = 0x20028,
    //OSSpecific          = 0x20029
}

// Does a semihosting operation.
fn semihost(op: Operation, param: Field) -> Field {
    let op = op as u32;
    let result: Field;

    unsafe {
        #[cfg(target_pointer_width = "64")]
        asm!(
            "hlt #0xf000",
            in("w0") op,
            in("x1") param,
            lateout("x0") result,
            options(preserves_flags)
        );
        #[cfg(target_pointer_width = "32")]
        asm!(
            "svc 0x123456",
            inout("r0") op => result,
            in("r1") param,
            options(preserves_flags)
        );
        // The T32 instruction set uses different instructions for this:
        //  `SVC 0xab` on processors with the A and R profiles, and
        //  `BKPT 0xab` on processors with the M profile.
        // I don't think the kernel will ever use T32, though.
    }

    result
}

// Returns the value of the C library's `errno` variable.
fn errno() -> Field {
    semihost(Operation::ErrNo, 0)
}

/// Attempts to shut down by telling the host we're done.
pub fn exit(error_code: i32) {
    #[repr(C)]
    struct Param {
        reason: ExitReason,
        subcode: Field
    }
    assert_eq_size!(Param, [Field; 2]);

    #[cfg(target_pointer_width = "64")] {
        let param = Param {
            reason: ExitReason::ApplicationExit,
            subcode: error_code as Field
        };
        semihost(Operation::Exit, &param as *const _ as Field);
    } #[cfg(target_pointer_width = "32")] {
        if (*EXTENSIONS).contains(Extensions::ExitExtended) {
            let param = Param {
                reason: ExitReason::ApplicationExit,
                subcode: error_code as Field
            };
            semihost(Operation::ExitExt, &param as *const _ as Field);
        } else {
            semihost(Operation::Exit, ExitReason::ApplicationExit);
        }
    }
}
