/* Copyright (c) 2022 Jeremy Davis (jeremydavis519@gmail.com)
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

//! This crate defines the [`include_idl!`] macro, which converts all the text in a given IDL file
//! into valid Rust code. It is also possible to use [`parse_idl!`], which accepts a string
//! containing IDL source code (a so-called "IDL fragment") rather than a filename. The grammar
//! and semantics came from the standard at [https://webidl.spec.whatwg.org/].
//!
//! Here are all the IDL concepts that need to be changed when converted into Rust:
//!
//! ## Identifiers
//! * In accordance with the specification, any leading `_` is stripped from each identifier.
//! * Any identifier beginning with `-` is changed to one beginning with `___` (which is not
//!   stripped).
//! * Any identifiers that are the same (i.e. the names of an overloaded operation or constructor)
//!   are distinguished with a suffix. The suffix consists of the letter `O` followed by the number
//!   of overloads defined before this one. The first overload does not receive a suffix at all. For
//!   instance, the following IDL fragment and Rust code are equivalent:
//!   ```text
//!   interface CanvasDrawPathExcerpt {
//!     undefined stroke();
//!     undefined stroke(Path2D path);
//!   };
//!   ```
//!   ```
//!   trait CanvasDrawPathExcerpt {
//!       fn stroke(&mut self);
//!       fn strokeO1(&mut self, path: Box<dyn Path2D>);
//!   }
//!   # trait Path2D {}
//!   ```
//!
//! ## Keywords
//! The following keywords map directly to Rust:
//! * `interface` -> `pub trait`
//! * `namespace` -> `pub mod`
//! * `dictionary` -> `pub struct`
//! * `enum _` -> `static _: [&str; n]`
//! * `typedef a b` -> `type b = a`
//! * `null` -> `None`
//! * `constructor(...)` -> `fn constructor(&mut self, ...)`[^1]
//! * `readonly` -> `const` (where applicable, e.g. `const fn`)
//! * `iterable<V>` -> `fn _iter<'a>(&mut self) -> Box<dyn Iterator<Item = &'a mut V>>`
//! * `iterable<K, V>` -> `fn _iter<'a>(&mut self) -> Box<dyn Iterator<Item = &'a mut KeyValue<'a>>>`[^2]
//! * `stringifier` -> `fn toString(&mut self)`[^3]
//! * `getter`, `setter`, and `deleter` are ignored; their operations are treated like regular
//!   operations.
//!
//! [^1]: If interface `Foo` has a constructor, it is expected that every method `Bar::constructor`,
//!   where `Bar: Foo`, will call `(self as Foo).constructor()`. IDL uses standard OOP constructors,
//!   but Rust requires us to do it explicitly.
//! [^2]: For an interface named `Foo`, `KeyValue` is defined in the module `_Foo` as follows:
//!   ```
//!   pub struct KeyValue<'a> {
//!       key: K,
//!       value: &'a mut V
//!   }
//!   # type K = (); type V = ();
//!   ```
//! [^3]: When used before an attribute, the `stringifier` keyword generates an appropriate default
//!   implementation.
//!
//! ## Attributes
//! A read-write attribute `foo` with type `T` is converted into an accessor (`fn foo(&self) -> T`)
//! and a mutator (`fn _set_foo(&mut self, value: T)`). A read-only attribute only gets the accessor.
//!
//! In the special case of an attribute that is a member of a namespace (which must be read-only),
//! ***** FIXME: What happens then??? *****
//!
//! ## Types
//! The built-in types are mapped as follows:
//! * `undefined` -> `()`
//! * `any` -> `Rc<dyn Any>`
//! * `object` -> `Rc<Object>`[^4]
//! * `boolean` -> `bool`
//! * `byte` -> `i8`
//! * `octet` -> `u8`
//! * `bigint` -> `Rc<BigInt>`[^4]
//! * `short` -> `i16`
//! * `unsigned short` -> `u16`
//! * `long` -> `i32`
//! * `unsigned long` -> `u32`
//! * `long long` -> `i64`
//! * `unsigned long long` -> `u64`
//! * `float` -> `f32`
//! * `double` -> `f64`
//! * `restricted float` -> `Restricted<f32>`[^4]
//! * `restricted f64` -> `Restricted<f64>`[^4]
//! * `Int8Array` -> `Rc<Vec<i8>>`
//! * `Int16Array` -> `Rc<Vec<i16>>`
//! * `Int32Array` -> `Rc<Vec<i32>>`
//! * `Uint8Array` -> `Rc<Vec<u8>>`
//! * `Uint16Array` -> `Rc<Vec<u16>>`
//! * `Uint32Array` -> `Rc<Vec<u32>>`
//! * `BigInt64Array` -> `Rc<Vec<i64>>`
//! * `BigUint64Array` -> `Rc<Vec<u64>>`
//! * `Float32Array` -> `Rc<Vec<f32>>`
//! * `Float64Array` -> `Rc<Vec<f64>>`
//! * `ByteString` -> `Rc<ByteString>`[^4]
//! * `DOMString` -> `Rc<DomString>`[^4]
//! * `USVString` -> `Rc<String>`
//! * `sequence<...>` -> `Rc<Vec<...>>`
//! * Any interface type `Foo` -> `Rc<dyn Foo>`
//!
//! Nullable types like `long?` are represented as optional types like `Option<i32>`.
//!
//! Variadic types like `long...` are represented as array slices like `&[i32]`.
//!
//! [^4]: To define these types, call `def_idl_types!()` where you want them to be defined. It
//!   should be called only once.
//!
//! ### Union types
//! Each union type is translated into an enum. The name of this enum is derived from the IDL union
//! type name as follows:
//! 1. For each union type `U` contained within this type, run this algorithm on `U` first and
//!   replace its type with the result, followed by an underscore.
//! 2. Replace every block of whitespace in the type name with a single underscore.
//! 3. Prepend `_Union_` to the whole type name.
//! 4. Remove the outer set of parentheses.
//!
//! For instance, the union type `(long or boolean or (string or unsigned long long))` is rendered
//! in Rust as `_Union_long_or_boolean_or__Union_string_or_unsigned_long_long_`.
//!
//! In order to actually use the type, it must be accessed from a generated module that accompanies
//! the trait or struct generated from the IDL interface or dictionary. This module has the same
//! name as the trait or struct except an underscore has been prepended to it. For example:
//! ```text
//! interface EventTarget {
//!   undefined addEventListener(
//!     DOMString type,
//!     EventListener? callback,
//!     optional (AddEventListenerOptions or boolean) options = {}
//!   );
//! };
//!
//! interface EventListener { /* ... */ };
//!
//! dictionary AddEventListenerOptions { /* ... */ };
//! ```
//! ```
//! struct Element { /* ... */ }
//!
//! impl EventTarget for Element {
//!     fn add_event_listener(
//!             r#type:   Vec<u16>,
//!             callback: Option<Box<dyn EventListener>>,
//!             options:  _EventTarget::_Union_AddEventListenerOptions_or_boolean
//!     ) { /* ... */ }
//! }
//! # trait EventTarget {
//! #     fn add_event_listener(
//! #         r#type: Vec<u16>,
//! #         callback: Option<Box<dyn EventListener>>,
//! #         options: _EventTarget::_Union_AddEventListenerOptions_or_boolean
//! #     );
//! # }
//! # trait EventListener {}
//! # mod _EventTarget { pub enum _Union_AddEventListenerOptions_or_boolean {} }
//! ```
//!
//! ## Dictionaries
//! In accordance with Rust's pattern of polymorphism by composition rather than by inheritance,
//! when a dictionary inherits from another in IDL, it instead contains the other in Rust. The
//! parent's members can be accessed through the child's `_super` member, which is an instance of
//! the parent.
//!
//! Every `required` dictionary member, and every optional dictionary member with a default value,
//! has the type described in the _Types_ section above. The type of an optional member with no
//! default value is wrapped in `Option<...>`.
//!
//! Each optional member of a dictionary comes with an associated function that returns its default
//! value. Additionally, if every member of a dictionary is optional, the whole struct implements
//! `Default`. For instance, the following IDL fragment and Rust code are equivalent:
//! ```text
//! dictionary Foo {
//!   long bar = 42;
//!   long baz;
//! };
//! ```
//! ```
//! pub struct Foo {
//!     pub bar: i32,
//!     pub baz: Option<i32>
//! }
//!
//! impl Foo {
//!     pub fn bar() -> i32 { 42 }
//!     pub fn baz() -> Option<i32> { None }
//! }
//!
//! impl Default for Foo {
//!     fn default() -> Self {
//!         Self {
//!             bar: Self::bar(),
//!             baz: Self::baz()
//!         }
//!     }
//! }
//! ```
//!
//! ## Arguments
//! Since Rust doesn't have optional parameters, any default values of IDL arguments are ignored.
//!
//! ## Extended Attributes
//! Extended attributes defined in the WebIDL standard are handled directly by this crate. All
//! others generate calls to macros that the client code needs to define.
//! * The macro name is the extended attribute's name, prepended with `"idlea_"`.
//! * Any extended attribute that contains a list of things (identifiers or arguments) will have
//!   an extra comma at the end of the list, even if that comma was not present in IDL. This makes
//!   processing the list a bit easier.
//! * If the extended attribute takes arguments with IDL types, it is converted to a Rust syntax
//!   in the same way as an operation is converted to a method declaration. For instance,
//!   `[Foo(long value, bool setNotGet)]` is converted to `[Foo(value: i32, setNotGet: bool,)]`.
//! * After the conversion, if any, the extended attribute, including surrounding brackets, is
//!   passed to the macro, followed by the Rust version of the annotated IDL item.
//!
//! If multiple extended attributes appear on the same definition, they are applied in order from
//! left to right.
//!
//! For example, the following IDL fragment and Rust snippet are equivalent:
//! ```text
//! interface ElementFragment {
//!   [CEReactions, AutoImpl = genericSetAttr(DOMString qualifiedName, DOMString value)]
//!   undefined setAttribute(DOMString qualifiedName, DOMString value);
//! }
//! ```
//! ```
//! # macro_rules! idlea_AutoImpl { ([$($attr:tt)*] $($tts:tt)*) => { $($tts)* }; }
//! # macro_rules! idlea_CEReactions { ([$($attr:tt)*] $($tts:tt)*) => { $($tts)* }; }
//! pub trait ElementFragment {
//!     idlea_AutoImpl! {
//!         [AutoImpl = genericSetAttr(qualified_name: Vec<u16>, value: Vec<u16>,)]
//!         idlea_CEReactions! {
//!             [CEReactions]
//!             fn setAttribute(&mut self, qualifiedName: Vec<u16>, value: Vec<u16>);
//!         }
//!     }
//! }
//! ```

#![feature(proc_macro_expand)]
#![feature(proc_macro_quote)]

use proc_macro::{Ident, Span, TokenStream, TokenTree, quote};

mod float;
mod parser;

#[proc_macro]
pub fn include_idl(tts: TokenStream) -> TokenStream {
    parse_idl(quote!(::core::include_str!($tts)))
}

#[proc_macro]
pub fn parse_idl(tts: TokenStream) -> TokenStream {
    let tts = tts.expand_expr()
        .expect("expected a string literal");

    let mut tts = tts.into_iter();
    let expr = tts.next()
        .expect("expected a string literal");
    assert!(tts.next().is_none(), "expected only one argument");

    match litrs::StringLit::try_from(expr) {
        Ok(lit) => parser::parse_definitions(lit.value()),
        Err(e) => e.to_compile_error()
    }
}

#[proc_macro]
pub fn def_idl_types(tts: TokenStream) -> TokenStream {
    if !tts.is_empty() {
        return quote!(::core::compile_error!("`def_idl_types` expected 0 arguments");)
    }

    let mut tts = float::restricted_float();
    let byte_str_type = TokenTree::Ident(Ident::new_raw("ByteString", Span::call_site()));
    tts.extend(quote!(pub type $byte_str_type = ::alloc::vec::Vec<u8>;));
    let dom_str_type = TokenTree::Ident(Ident::new_raw("DomString", Span::call_site()));
    tts.extend(quote!(pub type $dom_str_type = ::alloc::vec::Vec<u16>;));
    tts
}
