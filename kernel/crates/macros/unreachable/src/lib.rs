#![no_std]

/// This macro should be used everywhere code can be proven to be unreachable. Rust provides the
/// `unreachable` macro and the `unreachable_unchecked` function for this purpose. The former
/// panics, and the latter guarantees undefined behavior (which can allow more optimizations).
/// This macro, `unreachable_debug`, uses the panicking `unreachable` in debug mode and
/// `unreachable_unchecked` in release mode.
///
/// Since undefined behavior can easily cause bugs in seemingly unrelated parts of the code, this
/// macro takes a string literal as a required argument, which should be a proof that the code is
/// actually unreachable. The proof is included in the panic message if, in debug mode, it turns
/// out to be false.
///
/// # Safety
/// Since calling this macro in release mode is undefined behavior, it requires an `unsafe` block.
/// And, of course, since it is undefined behavior, the branch that includes this macro must never
/// be reached.
///
/// # Example
/// ```
/// # #[macro_use] extern crate macros_unreachable; fn main() {
/// let opt = Some(42);
/// assert_eq!(opt.unwrap_or_else(
///     || unsafe { unreachable_debug!("opt was just defined as Some.") }
/// ), 42);
/// # }
/// ```
#[macro_export]
macro_rules! unreachable_debug {
    ( $proof:expr ) => {
        {
            #[cfg(debug_assertions)] {
                // We use an inline function here to tell Rust that the call is unsafe.
                #[inline(always)]
                unsafe fn unreachable() -> ! {
                    unreachable!("flawed proof of unreachability: `{}`", $proof)
                }
                unreachable()
            }

            #[cfg(not(debug_assertions))] {
                core::hint::unreachable_unchecked()
            }
        }
    };
}
