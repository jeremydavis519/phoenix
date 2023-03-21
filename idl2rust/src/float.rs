/* Copyright (c) 2022-2023 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This module defines extensions to Rust's floating point numbers that are necessary for type
//! safety when working with IDL's type system.

use {
    proc_macro2::{TokenStream, TokenTree, Ident, Span},
    quote::{ToTokens, quote},
};

pub(crate) fn restricted_float() -> TokenStream {
    // These identifiers need to be defined manually to avoid mixed-site macro hygiene.
    let restricted = TokenTree::Ident(Ident::new_raw("Restricted", Span::call_site()));
    let try_new = TokenTree::Ident(Ident::new_raw("try_new", Span::call_site()));
    let new_unchecked = TokenTree::Ident(Ident::new_raw("new_unchecked", Span::call_site()));
    let get = TokenTree::Ident(Ident::new_raw("get", Span::call_site()));
    let float = TokenTree::Ident(Ident::new_raw("Float", Span::mixed_site()));

    quote!(
        /// A floating-point number that is guaranteed to be finite (i.e. not infinite and not NaN).
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub struct #restricted<T: #float>(T);

        impl<T: #float> #restricted<T> {
            // FIXME: Make these all `const fn` when https://github.com/rust-lang/rust/issues/72505 is stabilized.

            /// Constructs a restricted float from the given unrestricted one, if it is finite. Returns
            /// `None` otherwise.
            pub fn #try_new(val: T) -> ::core::option::Option<Self> {
                if val.is_finite() {
                    Some(Self(val))
                } else {
                    None
                }
            }

            /// Constructs a restricted float from the given unrestricted one. Does not check whether the
            /// value is finite, so misusing this leads to undefined behavior.
            pub unsafe fn #new_unchecked(val: T) -> Self {
                Self(val)
            }

            /// Unwraps the restricted float into an unrestricted float, which is necessary for doing any
            /// calculations with it.
            pub fn #get(self) -> T {
                self.0
            }
        }

        // Implementation details
        #[doc(hidden)]
        pub trait #float: Copy {
            fn is_finite(self) -> bool;
        }

        impl #float for ::core::primitive::f32 {
            fn is_finite(self) -> bool { ::core::primitive::f32::is_finite(self) }
        }

        impl #float for ::core::primitive::f64 {
            fn is_finite(self) -> bool { ::core::primitive::f64::is_finite(self) }
        }
    )
}
