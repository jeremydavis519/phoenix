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

// TODO: Replace $()* with $()? where appropriate.

/// This macro is defined to reduce the amount of boilerplate code associated with dispatching
/// requests for concrete strings to the appropriate language submodule. With its help, the crate's
/// root module only needs to define each language and string once.
macro_rules! boilerplate {
    (
        @internal Language {
            $($lang_variant:ident : $lang_module:ident : $lang_text:expr; $lang_feature:expr),*
        }
    ) => {
        #[cfg(feature = "all_languages")]
        static CURRENT_LANG: core::sync::atomic::AtomicU64 =
            core::sync::atomic::AtomicU64::new(boilerplate!(@first_lang_variant $($lang_variant)*) as u64);

        $(
            #[cfg(any(feature = $lang_feature, feature = "all_languages"))]
            mod $lang_module;
            #[cfg(all(feature = $lang_feature, not(feature = "all_languages")))]
            static CURRENT_LANG: core::sync::atomic::AtomicU64 =
                core::sync::atomic::AtomicU64::new(Language::$lang_variant as u64);
        )*

        /// Represents a language to be used for displaying text.
        #[repr(u64)]
        #[derive(Debug)]
        pub enum Language {
            #[allow(missing_docs)]
            $($lang_variant),*
        }

        impl From<Language> for u64 {
            fn from(lang: Language) -> u64 {
                lang as u64
            }
        }

        impl core::convert::TryFrom<u64> for Language {
            type Error = InvalidLanguage;

            fn try_from(raw: u64) -> Result<Language, Self::Error> {
                match raw {
                    $(x if x == Language::$lang_variant as u64 => Ok(Language::$lang_variant),)*
                    x => Err(InvalidLanguage(x))
                }
            }
        }

        impl fmt::Display for Language {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", match *self {
                    $(Language::$lang_variant => $lang_text),*
                })
            }
        }

        /// An error type indicating that an attempt to convert a raw integer to a variant of the
        /// `Language` enum failed. The error message is only in English because if this error
        /// appears, the entire internationalization system may be broken, and we want to avoid
        /// failing even more catastrophically.
        ///
        /// Also, it doesn't implement `shared::std::error::Error` because the crate in which that trait is
        /// defined depends on this one. It shouldn't actually matter, though.
        #[derive(Debug)]
        pub struct InvalidLanguage(u64);

        impl fmt::Display for InvalidLanguage {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "attempted to convert the invalid number {} into a Language variant", self.0)
            }
        }
    };

    ( @first_lang_variant $lang_variant:ident $($extra:tt)* ) => { Language::$lang_variant };

    (
        @internal Text<'a> {
            $Language:ident,
            $($lang_variant:ident : $lang_module:ident),*
            |
            $(
                $(#[$text_va_attr:meta])*
                $text_variant:ident $(($($text_va_arg:ident : $text_va_type:ty),*))*
            ),*
        }
    ) => {
        /// Represents a string of text to be displayed in some language.
        #[derive(Debug)]
        pub enum Text<'a> {
            $(
                /// Can be `Display`ed like a string to show the relevant text in this language.
                $(#[$text_va_attr])*
                $text_variant $(($($text_va_type),*))*
            ),*
        }

        #[allow(irrefutable_let_patterns)]
        impl<'a> fmt::Display for Text<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                unsafe {
                    boilerplate!(
                        @internal Text_dispatch {
                            $Language,
                            *self, f,
                            $($lang_variant: $lang_module),*
                            |
                            $($text_variant $(($($text_va_arg),*))*;)*
                        }
                    )
                }
            }
        }
    };

    (
        @internal Text_dispatch {
            $Language:ident,
            $self:expr, $f:expr,
            $($lang_variant:ident : $lang_module:ident),*
            |
        }
    ) => {
        unreachable_debug!(
            "This is the unconditional branch after an exhaustive series of conditional branches on text variants."
        )
    };

    (
        @internal Text_dispatch {
            $Language:ident,
            $self:expr, $f:expr,
            $($lang_variant:ident : $lang_module:ident),*
            |
            $text_variant:ident $(($($text_va_arg:ident),*))* ;
            $($extra:tt)*
        }
    ) => {
        if let Text::$text_variant $(($(ref $text_va_arg),*))* = $self {
            let current_lang = <$Language as core::convert::TryFrom<u64>>::try_from($crate::CURRENT_LANG.load(core::sync::atomic::Ordering::Acquire))
                .expect("failed to change the current language");
            boilerplate!(@internal Text_dispatch2 {
                $Language,
                $f, current_lang,
                $text_variant $(($($text_va_arg),*))*
                |
                $($lang_variant: $lang_module;)*
            })
        } else {
            boilerplate!(@internal Text_dispatch {
                $Language,
                $self, $f,
                $($lang_variant: $lang_module),*
                |
                $($extra)*
            })
        }
    };

    (
        @internal Text_dispatch2 {
            $Language:ident,
            $f:expr, $current_lang:expr,
            $text_variant:ident $(($($text_va_arg:ident),*))*
            |
        }
    ) => {
        unreachable_debug!(
            "This is the unconditional branch after an exhaustive series of conditional branches on language variants."
        )
    };

    (
        @internal Text_dispatch2 {
            $Language:ident,
            $f:expr, $current_lang:expr,
            $text_variant:ident $(($($text_va_arg:ident),*))*
            |
            $lang_variant:ident : $lang_module:ident ;
            $($extra:tt)*
        }
    ) => {
        if let $Language::$lang_variant = $current_lang {
            write!($f, "{}", $lang_module::Text::$text_variant $(($($text_va_arg),*))*)
        } else {
            boilerplate!(@internal Text_dispatch2 {
                $Language,
                $f, $current_lang,
                $text_variant $(($($text_va_arg),*))*
                |
                $($extra)*
            })
        }
    };

    (
        @internal impl<'a> Text<'a> {
            $($token:tt)*
        }
    ) => {
        impl<'a> Text<'a> {
            boilerplate! {
                @internal dispatch_impl_Text {
                    $($token)*
                }
            }
        }
    };

    (
        @internal dispatch_impl_Text {
            $Language:ident, $Text:ident,
            $($lang_variant:ident : $lang_module:ident),*
            |
        }
    ) => {};

    (
        @internal dispatch_impl_Text {
            $Language:ident, $Text:ident,
            $($lang_variant:ident : $lang_module:ident),*
            |
            $(#[$text_fn_attr:meta])*
            $vis:vis fn $text_function:ident ( $($text_fn_arg:ident : $text_fn_type:ty),* ) -> $text_fn_ret:ty;
            $($extra:tt)*
        }
    ) => {
        #[allow(missing_docs)]
        $(#[$text_fn_attr])*
        $vis fn $text_function($($text_fn_arg: $text_fn_type),*) -> $text_fn_ret {
            (match <$Language as core::convert::TryFrom<u64>>::try_from($crate::CURRENT_LANG.load(core::sync::atomic::Ordering::Acquire))
                    .expect("failed to get the current language") {
                $($Language::$lang_variant => $lang_module::Text::$text_function),*
            })($($text_fn_arg),*)
        }
        boilerplate!(
            @internal dispatch_impl_Text {
                $Language, $Text,
                $($lang_variant: $lang_module),*
                |
                $($extra)*
            }
        );
    };

    (
        pub enum Language {
            $($lang_variant:ident : $lang_module:ident : $lang_text:expr; feature = $lang_feature:expr),*
        }
        pub enum Text<'a> {
            $(
                $(#[$text_va_attr:meta])*
                $text_variant:ident $(($($text_va_arg:ident : $text_va_type:ty),*))*
            ),*
        }
        impl<'a> Text<'a> {
            $(
                $(#[$text_fn_attr:meta])*
                $vis:vis fn $text_function:ident ( $($text_fn_arg:ident : $text_fn_type:ty),* ) -> $text_fn_ret:ty;
            )*
        }
    ) => {
        boilerplate! {
            @internal Language {
                $($lang_variant: $lang_module: $lang_text; $lang_feature),*
            }
        }
        boilerplate! {
            @internal Text<'a> {
                Language,
                $($lang_variant: $lang_module),*
                |
                $(
                    $(#[$text_va_attr])*
                    $text_variant $(($($text_va_arg: $text_va_type),*))*
                ),*
            }
        }
        boilerplate! {
            @internal impl<'a> Text<'a> {
                Language, Text,
                $($lang_variant: $lang_module),*
                |
                $(
                    $(#[$text_fn_attr])*
                    $vis fn $text_function($($text_fn_arg: $text_fn_type),*) -> $text_fn_ret;
                )*
            }
        }
    };
}
