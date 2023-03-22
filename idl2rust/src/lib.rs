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

//! Transpile IDL fragments into Rust source code at compile time.
//!
//! This crate provides procedural macros that read IDL fragments and produce equivalent Rust code,
//! ready to be implemented. The dialect of IDL used here is based on [Web IDL], but with a few
//! changes:
//! - Mixins and partial definitions are not supported.
//! - Everything is considered a valid identifier if it matches the [`identifier` regular
//!   expression], even if the token appears elsewhere in the grammar, as long as there is no local
//!   ambiguity.
//!
//! [Web IDL]: https://webidl.spec.whatwg.org/
//! [`identifier` regular expression]: https://webidl.spec.whatwg.org/#prod-identifier
//!
//! Details of the conversion are listed below:
//!
//! ## Definitions
//! The different types of definitions are converted as follows:
//! - `callback X = T(/* args */)` -> `pub type X = Box<dyn FnMut(/* arg types */) -> T>`
//! - `callback interface X`       -> `pub struct X { /* operations */ }` and `pub mod _X { /* constants */ }`
//!   - Operations are represented as fields with the type `Box<dyn FnMut(/* arg types */) -> /* return type */>`.
//! - `dictionary X`               -> `pub struct X`
//! - `enum X`                     -> `pub mod X { pub static VALUES: Vec<DomString> = ...; }`
//! - `interface X`                -> `pub trait X { /* non-constants */ }` and `pub mod _X { /* constants */ }`
//! - `namespace X`                -> `pub mod X`
//!   - The namespace's attributes and operations must be defined by the implementor in a module
//!     called `_X`; `X` only contains stubs that call the functions in `_X`.
//! - `typedef T U` -> `pub type U = T;`
//!
//! ## Identifiers
//! * In accordance with the specification, any leading `_` is stripped from each identifier.
//! * Any identifier beginning with `-` is changed to one beginning with `___` (which is not
//!   stripped). /* FIXME */
//! * Any identifiers that are the same (i.e. the names of an overloaded operation or constructor)
//!   are distinguished with a prefix. The prefix consists of an underscore, the letter `O`, the
//!   number of overloads defined before this one, and another underscore. The first overload does
//!   not receive a prefix at all. For instance, the following IDL fragment and Rust code are
//!   equivalent:
//!   ```idl
//!   interface CanvasDrawPathExcerpt {
//!     undefined stroke();
//!     undefined stroke(Path2D path);
//!   };
//!   ```
//!   ```
//!   trait CanvasDrawPathExcerpt {
//!       fn stroke(&self);
//!       fn _O1_stroke(&self, path: Box<dyn Path2D>);
//!   }
//!   # trait Path2D {}
//!   ```
//!
//! ## Keywords
//! Keywords are translated as follows:
//! * `interface`        -> `pub trait`
//! * `namespace`        -> `pub mod`
//! * `dictionary`       -> `pub struct`
//! * `enum _`           -> `static _: [&str; n]`
//! * `typedef a b`      -> `type b = a`
//! * `null`             -> `None`
//! * `constructor(...)` -> `fn constructor(...) -> Self where Self: Sized`[^1]
//! * `readonly`         -> `const` (where applicable, e.g. `const fn`)
//! * `iterable<V>`      -> `fn _iter(&self) -> Box<dyn Iterator<Item = &mut V> + '_>`
//! * `iterable<K, V>`   -> `fn _iter(&self) -> Box<dyn Iterator<Item = &mut KeyValue<'_>> + '_>`[^2]
//! * `stringifier`      -> `fn toString(&self)`[^3]
//! * `getter`, `setter`, and `deleter` are ignored; their operations are treated like regular
//!   operations.
//!
//! [^1]: If interface `Foo` has a constructor, it is expected that every method `Bar::constructor`,
//!   where `Bar: Foo`, will call `Foo::constructor()`. IDL uses standard OOP constructors, but
//!   Rust requires us to do it explicitly.
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
//! and a mutator (`fn _set_foo(&self, value: T)`). A read-only attribute only gets the accessor.
//!
//! ## Types
//! Types are mapped as follows:
//! * `undefined`               -> `()`
//! * `any`                     -> `Rc<dyn Any>`
//! * `object`                  -> `Rc<Object>`[^4]
//! * `boolean`                 -> `bool`
//! * `byte`                    -> `i8`
//! * `octet`                   -> `u8`
//! * `bigint`                  -> `Rc<BigInt>`[^4]
//! * `short`                   -> `i16`
//! * `unsigned short`          -> `u16`
//! * `long`                    -> `i32`
//! * `unsigned long`           -> `u32`
//! * `long long`               -> `i64`
//! * `unsigned long long`      -> `u64`
//! * `float`                   -> `f32`
//! * `double`                  -> `f64`
//! * `restricted float`        -> `Restricted<f32>`[^4]
//! * `restricted f64`          -> `Restricted<f64>`[^4]
//! * `Int8Array`               -> `Rc<Vec<i8>>`
//! * `Int16Array`              -> `Rc<Vec<i16>>`
//! * `Int32Array`              -> `Rc<Vec<i32>>`
//! * `Uint8Array`              -> `Rc<Vec<u8>>`
//! * `Uint16Array`             -> `Rc<Vec<u16>>`
//! * `Uint32Array`             -> `Rc<Vec<u32>>`
//! * `BigInt64Array`           -> `Rc<Vec<i64>>`
//! * `BigUint64Array`          -> `Rc<Vec<u64>>`
//! * `Float32Array`            -> `Rc<Vec<f32>>`
//! * `Float64Array`            -> `Rc<Vec<f64>>`
//! * `ByteString`              -> `Rc<ByteString>`[^4]
//! * `DOMString`               -> `Rc<DomString>`[^4]
//! * `USVString`               -> `Rc<String>`
//! * `Promise<T>`              -> `Box<dyn Future<Output = T>>`
//! * `sequence<...>`           -> `Rc<Vec<...>>`
//! * Any interface type `Foo`  -> `Rc<dyn Foo>`
//! * Any dictionary type `Foo` -> `Rc<Foo>`
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
//! name as the trait or struct except an underscore has been prepended to it. The union is defined
//! in that module as an enum whose variants are named after the variant types of the union in IDL.
//!
//! For example:
//! ```idl
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
//!             options:  _EventTarget::_Union_AddEventListenerOptions_or_boolean,
//!     ) {
//!         use _EventTarget::_Union_AddEventListenerOptions_or_boolean::*;
//!         match options {
//!             AddEventListenerOptions(opts) => /* ... */,
//!             boolean(b)                    => /* ... */,
//!         }
//!     }
//! }
//! # trait EventTarget {
//! #     fn add_event_listener(
//! #         r#type: Vec<u16>,
//! #         callback: Option<Box<dyn EventListener>>,
//! #         options: _EventTarget::_Union_AddEventListenerOptions_or_boolean,
//! #     );
//! # }
//! # trait EventListener {}
//! # struct AddEventListenerOptions {}
//! # mod _EventTarget {
//! #     pub enum _Union_AddEventListenerOptions_or_boolean {
//! #         AddEventListenerOptions(Rc<AddEventListenerOptions>),
//! #         boolean(bool),
//! #     }
//! # }
//! ```
//!
//! ## Arguments
//! Since Rust doesn't have optional parameters, any default values of IDL arguments are ignored.
//!
//! ## Dictionaries
//! Every `required` dictionary member, and every optional dictionary member with a default value,
//! has the type described in the _Types_ section above. The type of an optional member with no
//! default value is wrapped in `Option<...>`.
//!
//! Each optional member of a dictionary comes with an associated function that returns its default
//! value. Additionally, if every member of a dictionary is optional, the whole struct implements
//! `Default`. For instance, the following IDL fragment and Rust code are equivalent:
//! ```idl
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
//! ## Inheritance
//! If dictionary `A` inherits from dictionary `B` in IDL, then in Rust dictionary `A` contains a
//! field of type `B`, which is called `_super`.
//!
//! If interface `A` inherits from interface `B` in IDL, the same is true of the Rust traits. In
//! addition, every trait has a method called `_super`, which should return an object that will
//! handle any other methods that are not overridden. (This is analogous to coercing an object to
//! an instance of its superclass in C++, Java, etc--hence the name.) The type of the returned
//! object is defined as `Super`.
//!
//! For example, the following IDL fragment:
//! ```idl
//! interface Node {
//!     Node parentNode();
//! }
//!
//! interface Element : Node {}
//! ```
//! produces the following Rust code:
//! ```
//! #[allow(non_snake_case)]
//! pub trait Node {
//!     fn _super(&self) -> ::alloc::rc::Rc<dyn Node> {
//!         panic!("attempted to find the supertrait of a base trait")
//!     }
//!     fn parentNode(&self) -> ::alloc::rc::Rc<dyn Node> {
//!         self._super().parentNode()
//!     }
//! }
//!
//! #[allow(non_snake_case)]
//! pub trait Element: Node {
//!     fn _super(&self) -> ::alloc::rc::Rc<dyn Element> {
//!         panic!("attempted to find the supertrait of a base trait")
//!     }
//! }
//! ```
//!
//! An implementor of `Element` should override `Node::Super`, `Node::_super`, and any other
//! methods whose behavior is different than in the case of a generic `Node` but should leave all
//! other methods alone to follow the principle of DRY.
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
//! ```idl
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
//!         [AutoImpl = genericSetAttr(qualified_name: Rc<DomString>, value: Rc<DomString>,)]
//!         idlea_CEReactions! {
//!             [CEReactions]
//!             fn setAttribute(&self, qualifiedName: Rc<DomString>, value: Rc<DomString>) -> ();
//!         }
//!     }
//! }
//! ```
//!
//! ## Type-Casting
//! Casting between traits defined by IDL interfaces is provided by the `intertrait` crate. Every
//! such trait is marked as a possible source for casting. See the `intertrait` crate's
//! documentation for how to declare target traits.

#![feature(proc_macro_expand)]

use {
    proc_macro::TokenStream,
    proc_macro2::{Ident, Span, TokenTree},
    quote::{ToTokens, quote},
};

mod ast;
mod float;
mod parser;

#[proc_macro]
pub fn include_idl(ts: TokenStream) -> TokenStream {
    let ts = proc_macro2::TokenStream::from(ts);
    parse_idl(quote!(::core::include_str!(#ts)).into())
}

#[proc_macro]
pub fn parse_idl(ts: TokenStream) -> TokenStream {
    let Ok(ts) = ts.expand_expr() else {
        return quote!(::core::compile_error!("expected a string literal");).into()
    };

    let mut ts = ts.into_iter();
    let Some(expr) = ts.next() else {
        return quote!(::core::compile_error!("expected a string literal");).into()
    };
    if !ts.next().is_none() {
        return quote!(::core::compile_error!("expected only one argument");).into()
    }

    let input = match litrs::StringLit::try_from(expr) {
        Ok(lit) => lit,
        Err(e) => return e.to_compile_error(),
    };

    match parser::parse(input.value()) {
        Ok(ast) => ast.into_token_stream().into(),
        Err(e) => {
            let s = String::from("IDL syntax error: ") + &e.to_string();
            quote!(::core::compile_error!(#s);).into()
        },
    }
}

#[proc_macro]
pub fn define_idl_types(ts: TokenStream) -> TokenStream {
    if !ts.is_empty() {
        return quote!(::core::compile_error!("expected 0 arguments");).into()
    }

    let mut ts = float::restricted_float();
    let byte_str_type = TokenTree::Ident(Ident::new_raw("ByteString", Span::call_site()));
    ts.extend(quote!(pub type #byte_str_type = ::alloc::vec::Vec<u8>;));
    let dom_str_type = TokenTree::Ident(Ident::new_raw("DomString", Span::call_site()));
    ts.extend(quote!(pub type #dom_str_type = ::alloc::vec::Vec<u16>;));
    ts.into()
}
