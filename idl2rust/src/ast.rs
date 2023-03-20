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

use {
    proc_macro2::{
        Ident,
        Span,
        TokenStream,
    },
    quote::{
        ToTokens,
        TokenStreamExt,
        format_ident,
        quote,
    },
};

pub struct Definitions<'a> {
    pub defs: Vec<(ExtendedAttributes<'a>, Definition<'a>)>
}

pub enum Definition<'a> {
    CallbackFunction(CallbackFunction<'a>),
    CallbackInterface(CallbackInterface<'a>),
    Dictionary(Dictionary<'a>),
    Enum(Enum<'a>),
    Interface(Interface<'a>),
    Namespace(Namespace<'a>),
    Typedef(Typedef<'a>),
}

pub struct Interface<'a> {
    pub ident: Identifier<'a>,
    pub inheritance: Option<Identifier<'a>>,
    pub members: Vec<(ExtendedAttributes<'a>, InterfaceMember<'a>)>,
}

pub enum InterfaceMember<'a> {
    Attribute(Attribute<'a>),
    Const(Const<'a>),
    Constructor(Vec<(ExtendedAttributes<'a>, Argument<'a>)>),
    Iterable(Iterable<'a>),
    Maplike(Maplike<'a>),
    Operation(ExtendedAttributes<'a>, Operation<'a>),
    Setlike(Setlike<'a>),
    StaticMember(StaticMember<'a>),
    Stringifier(Stringifier<'a>),
}

pub struct CallbackInterface<'a> {
    pub ident: Identifier<'a>,
    pub members: Vec<(ExtendedAttributes<'a>, CallbackInterfaceMember<'a>)>,
}

pub enum CallbackInterfaceMember<'a> {
    Const(Const<'a>),
    Operation(Operation<'a>),
}

pub struct Dictionary<'a> {
    pub ident: Identifier<'a>,
    pub inheritance: Option<Identifier<'a>>,
    pub members: Vec<(ExtendedAttributes<'a>, DictionaryMember<'a>)>,
}

pub struct DictionaryMember<'a> {
    pub ty: (ExtendedAttributes<'a>, SimpleType<'a>),
    pub ident: Identifier<'a>,
    pub default: Option<DefaultValue<'a>>, // None if required, Some(Undefined) if optional with no default
}

pub struct Namespace<'a> {
    pub ident: Identifier<'a>,
    pub members: Vec<(ExtendedAttributes<'a>, NamespaceMember<'a>)>,
}

pub enum NamespaceMember<'a> {
    Attribute(Attribute<'a>),
    Const(Const<'a>),
    Operation(Operation<'a>),
}

pub struct Enum<'a> {
    pub ident: Identifier<'a>,
    pub values: Vec<&'a str>,
}

pub struct Typedef<'a> {
    pub ty: (ExtendedAttributes<'a>, SimpleType<'a>),
    pub ident: Identifier<'a>,
}

pub struct CallbackFunction<'a> {
    pub ident: Identifier<'a>,
    pub ty: SimpleType<'a>,
    pub params: Vec<(ExtendedAttributes<'a>, Argument<'a>)>,
}

pub struct Const<'a> {
    pub ty: SimpleNonnullableType<'a>,
    pub ident: Identifier<'a>,
    pub value: ConstValue,
}

#[derive(Debug, Clone, Copy)]
pub enum ConstValue {
    Bool(bool),
    Float(f64),
    Int(i128),
}

#[derive(Debug, Clone)]
pub enum Type<'a> {
    Simple(SimpleType<'a>),
    Variadic(SimpleType<'a>),
}

#[derive(Debug, Clone)]
pub struct SimpleType<'a> {
    pub ty: SimpleNonnullableType<'a>,
    pub nullable: bool,
}

#[derive(Debug, Clone)]
pub enum SimpleNonnullableType<'a> {
    Any,
    ArrayBuffer,
    Boolean,
    BigInt,
    DataView,
    Float {
        ty: FloatType,
        restricted: bool,
    },
    FrozenArray(ExtendedAttributes<'a>, Box<SimpleType<'a>>),
    Identifier(Identifier<'a>),
    ArrayViewFloat(FloatType),
    ArrayViewInt {
        ty: IntegerType,
        signed: bool,
    },
    ArrayViewUint8Clamped,
    Integer {
        ty: IntegerType,
        signed: bool,
    },
    Object,
    ObservableArray(ExtendedAttributes<'a>, Box<SimpleType<'a>>),
    Promise(Box<SimpleType<'a>>),
    Record {
        key: StringType,
        value: (ExtendedAttributes<'a>, Box<SimpleType<'a>>),
    },
    Sequence(ExtendedAttributes<'a>, Box<SimpleType<'a>>),
    String(StringType),
    Symbol,
    Undefined,
    Union(Vec<(ExtendedAttributes<'a>, SimpleType<'a>)>),
}

#[derive(Debug, Clone, Copy)]
pub enum IntegerType {
    Byte,
    Short,
    Long,
    LongLong,
}

#[derive(Debug, Clone, Copy)]
pub enum FloatType {
    Float,
    Double,
}

#[derive(Debug, Clone, Copy)]
pub enum StringType {
    ByteString,
    DomString,
    UsvString,
}

#[derive(Debug, Clone)]
pub struct Attribute<'a> {
    pub tag: AttributeTag,
    pub ty: (ExtendedAttributes<'a>, Type<'a>),
    pub name: Identifier<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum AttributeTag {
    ReadOnly,
    Inherit,
    None,
}

#[derive(Debug, Clone)]
pub struct AttributeRest<'a> {
    pub ty: (ExtendedAttributes<'a>, Type<'a>),
    pub name: Identifier<'a>,
}

impl<'a> Attribute<'a> {
    pub fn new(tag: AttributeTag, rest: AttributeRest<'a>) -> Self {
        Self {
            tag,
            ty: rest.ty,
            name: rest.name,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DefaultValue<'a> {
    Const(ConstValue),
    String(&'a str),
    EmptySequence,
    EmptyDictionary,
    Null,
    Undefined,
}

#[derive(Debug, Clone)]
pub struct Operation<'a> {
    pub ty: SimpleType<'a>,
    pub ident: Option<Identifier<'a>>,
    pub params: Vec<(ExtendedAttributes<'a>, Argument<'a>)>,
}

impl<'a> Operation<'a> {
    pub fn new(ty: SimpleType<'a>, rest: OperationRest<'a>) -> Self {
        Self {
            ty,
            ident: rest.ident,
            params: rest.params,
        }
    }
}

pub struct OperationRest<'a> {
    pub ident: Option<Identifier<'a>>,
    pub params: Vec<(ExtendedAttributes<'a>, Argument<'a>)>,
}

#[derive(Debug, Clone)]
pub struct Argument<'a> {
    pub ty: (ExtendedAttributes<'a>, Type<'a>),
    pub ident: Identifier<'a>,
    pub default: Option<DefaultValue<'a>>,
}

#[derive(Debug, Clone)]
pub enum Stringifier<'a> {
    Simple,
    Attribute(Attribute<'a>),
}

#[derive(Debug, Clone)]
pub enum StaticMember<'a> {
    Attribute(Attribute<'a>),
    Operation(Operation<'a>),
}

pub enum Iterable<'a> {
    Sync {
        key: Option<(ExtendedAttributes<'a>, SimpleType<'a>)>,
        value: (ExtendedAttributes<'a>, SimpleType<'a>),
    },
    Async {
        key: Option<(ExtendedAttributes<'a>, SimpleType<'a>)>,
        value: (ExtendedAttributes<'a>, SimpleType<'a>),
        args: Vec<(ExtendedAttributes<'a>, Argument<'a>)>,
    },
}

pub struct Maplike<'a> {
    pub readonly: bool,
    pub from: (ExtendedAttributes<'a>, SimpleType<'a>),
    pub to: (ExtendedAttributes<'a>, SimpleType<'a>),
}

pub struct Setlike<'a> {
    pub readonly: bool,
    pub ty: (ExtendedAttributes<'a>, SimpleType<'a>),
}

#[derive(Debug, Clone, Copy)]
pub struct Identifier<'a> {
    pub name: &'a str,
}

#[derive(Debug, Clone)]
pub struct ExtendedAttributes<'a> {
    pub attrs: Vec<&'a str>
}

impl ExtendedAttributes<'_> {
    pub fn new() -> Self {
        Self { attrs: Vec::new() }
    }
}

#[derive(Debug, Clone)]
pub enum ExtendedAttribute<'a> {
    NoArgs(Identifier<'a>),
    ArgList(Identifier<'a>, Vec<(ExtendedAttributes<'a>, Argument<'a>)>),
    Ident(Identifier<'a>, Identifier<'a>),
    Wildcard(Identifier<'a>),
    IdentList(Identifier<'a>, Vec<Identifier<'a>>),
    NamedArgList(Identifier<'a>, Identifier<'a>, Vec<(ExtendedAttributes<'a>, Argument<'a>)>),
}

// -----------------------------------------------
// Transpilation from AST to Rust
// -----------------------------------------------

impl ToTokens for Definitions<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        for (attrs, def) in self.defs.iter() {
            let def_ts = def.to_token_stream();
            attrs.apply(def_ts, ts);
        }
    }
}

impl ToTokens for Definition<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        match self {
            Self::CallbackFunction(x) => x.to_tokens(ts),
            Self::CallbackInterface(x) => x.to_tokens(ts),
            Self::Dictionary(x) => x.to_tokens(ts),
            Self::Enum(x) => x.to_tokens(ts),
            Self::Interface(x) => x.to_tokens(ts),
            Self::Namespace(x) => x.to_tokens(ts),
            Self::Typedef(x) => x.to_tokens(ts),
        }
    }
}

impl ToTokens for CallbackFunction<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ident = self.ident;
        let alias_ident = ident.as_type_ident();
        let ty = &self.ty;

        let params = self.params.iter().fold(
            TokenStream::new(),
            |mut params, (attrs, param)| {
                let (_, ty) = &param.ty;
                attrs.apply(ty.into_token_stream(), &mut params);
                params.append_all(quote!(,));
                params
            }
        );

        ts.append_all(quote!(
            pub type #ident = ::alloc::boxed::Box<dyn FnMut(#params) -> #ty>;
            pub type #alias_ident = #ident;
        ));
    }
}

impl ToTokens for CallbackInterface<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ident = self.ident;
        let mod_ident = ident.as_alt_ident();
        let alias_ident = ident.as_type_ident();

        let struct_members = self.members.iter().fold(
            TokenStream::new(),
            |mut members, (attr, member)| {
                match member {
                    CallbackInterfaceMember::Const(_) => {},
                    CallbackInterfaceMember::Operation(op) => op.to_tokens(&mut members),
                };
                members
            }
        );

        let mod_members = self.members.iter().fold(
            TokenStream::new(),
            |mut members, (attr, member)| {
                match member {
                    CallbackInterfaceMember::Const(c) => c.to_tokens(&mut members),
                    CallbackInterfaceMember::Operation(_) => {},
                };
                members
            }
        );

        #[cfg(feature = "debug")]
        let derive = quote!(#[derive(::core::fmt::Debug)]);
        #[cfg(not(feature = "debug"))]
        let derive = TokenStream::new();

        ts.append_all(quote!(
            #derive
            pub struct #ident {
                #struct_members
            }

            pub type #alias_ident = #ident;

            pub mod #mod_ident {
                #mod_members
            }
        ));
    }
}

impl ToTokens for Dictionary<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ident = self.ident;
        let alias_ident = ident.as_type_ident();

        let inheritance = match self.inheritance {
            Some(ref inheritance) => quote!(_super: #inheritance,),
            None => TokenStream::new(),
        };

        let members = self.members.iter().fold(
            TokenStream::new(),
            |mut ts, (attrs, member)| {
                attrs.apply(member.to_token_stream(), &mut ts);
                ts.append_all(quote!(,));
                ts
            },
        );

        let default_impl = if self.defaultable() {
            let super_default = match self.inheritance {
                Some(_) => quote!(_super: ::std::default::Default::default(),),
                None => TokenStream::new(),
            };
            let members_default = self.members.iter().fold(
                TokenStream::new(),
                |mut ts, (_, member)| {
                    let ident = member.ident;
                    let value = member.default_value().unwrap();
                    ts.append_all(quote!(#ident: #value,));
                    ts
                }
            );
            quote!(
                impl ::std::default::Default for #ident where #inheritance: ::std::default::Default {
                    fn default() -> Self {
                        Self {
                            #super_default
                            #members_default
                        }
                    }
                }
            )
        } else {
            TokenStream::new()
        };

        #[cfg(feature = "debug")]
        let derive = quote!(#[derive(::core::fmt::Debug)]);
        #[cfg(not(feature = "debug"))]
        let derive = TokenStream::new();

        ts.append_all(quote!(
            #[allow(non_snake_case)]
            #derive
            pub struct #ident {
                #inheritance
                #members
            }

            pub type #alias_ident = #ident;

            #default_impl
        ));
    }
}

impl Dictionary<'_> {
    fn defaultable(&self) -> bool {
        self.members.iter().all(|(_, member)| member.default_value().is_some())
    }
}

impl ToTokens for DictionaryMember<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ident = self.ident;
        let (attrs, ty) = &self.ty;

        ts.append_all(quote!(#ident:));
        let ty_ts = match self.default {
            None => quote!(#ty), // Required
            Some(_) => quote!(::core::option::Option<#ty>), // Optional
        };
        attrs.apply(ty_ts, ts);
    }
}

impl DictionaryMember<'_> {
    fn default_value(&self) -> Option<DefaultValue> {
        match self.default {
            Some(DefaultValue::Undefined) => None, // Optional, but no default value
            x => x,
        }
    }
}

impl ToTokens for Enum<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ident = self.ident;
        let type_ident = self.ident.as_type_ident();
        let mut values = TokenStream::new();
        values.append_terminated(self.values.iter(), quote!(,));

        #[cfg(feature = "debug")]
        let derive = quote!(#[derive(::core::fmt::Debug, ::core::clone::Clone)]);
        #[cfg(not(feature = "debug"))]
        let derive = quote!(#[derive(::core::clone::Clone)]);

        ts.append_all(quote!(
            pub mod #ident {
                pub static VALUES: ::alloc::vec::Vec<DOMString> = [#values];
            }

            #derive
            pub struct #type_ident(DOMString);
            impl ::core::convert::From<#type_ident> for DOMString {
                fn from(value: #type_ident) -> Self {
                    value.0
                }
            }
            impl ::core::convert::TryFrom<DOMString> for #type_ident {
                type Error = /* FIXME */;

                fn try_from(value: DOMString) -> ::core::result::Result<Self, Self::Error> {
                    #ident::VALUES.iter().find_map(|&s| if value == s { Ok(#type_ident(value)) } else { Err(/* FIXME */) })
                }
            }
        ));
    }
}

impl ToTokens for Interface<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ident = self.ident;
        let mod_ident = ident.as_alt_ident();
        let alias_ident = ident.as_type_ident();

        #[allow(unused_mut)] // Unused if we're not requiring Debug
        let mut bounds = match self.inheritance {
            Some(ref inheritance) => inheritance.into_token_stream(),
            None => TokenStream::new(),
        };

        #[cfg(feature = "debug")]
        if bounds.is_empty() {
            bounds = quote!(: ::core::fmt::Debug);
        } else {
            bounds.append_all(quote!(+ ::core::fmt::Debug));
        }

        let trait_members = self.members.iter().fold(
            TokenStream::new(),
            |mut ts, (attrs, member)| {
                let Some(member) = member.to_token_stream_in_trait() else { return ts };
                attrs.apply(member, &mut ts);
                ts
            },
        );

        let mod_members = self.members.iter().fold(
            TokenStream::new(),
            |mut ts, (attrs, member)| {
                let Some(member) = member.to_token_stream_in_mod() else { return ts };
                attrs.apply(member, &mut ts);
                ts
            }
        );

        ts.append_all(quote!(
            #[allow(non_snake_case)]
            pub trait #ident #bounds {
                fn _super(&self) -> ::alloc::rc::Rc<dyn #ident> {
                    panic!("attempted to find the supertrait of a base trait")
                }
                #trait_members
            }

            pub type #alias_ident = dyn #ident;

            pub mod #mod_ident {
                #mod_members
            }
        ));
    }
}

impl InterfaceMember<'_> {
    fn to_token_stream_in_trait(&self) -> Option<TokenStream> {
        match self {
            Self::Attribute(attr) => todo!(),
            Self::Const(c) => None,
            Self::Constructor(op) => {
                todo!()
            },
            Self::Iterable(i) => {
                todo!()
            },
            Self::Maplike(m) => {
                todo!()
            },
            Self::Operation(attrs, op) => {
                let mut ts = TokenStream::new();
                attrs.apply(op.into_token_stream(), &mut ts);
                Some(ts)
            },
            Self::Setlike(s) => {
                todo!()
            },
            Self::StaticMember(s) => todo!(),
            Self::Stringifier(s) => {
                todo!()
            },
        }
    }

    fn to_token_stream_in_mod(&self) -> Option<TokenStream> {
        match self {
            Self::Attribute(attr) => todo!(),
            Self::Const(c) => Some(c.into_token_stream()),
            Self::Constructor(op) => {
                todo!()
            },
            Self::Iterable(i) => {
                todo!()
            },
            Self::Maplike(m) => {
                todo!()
            },
            Self::Operation(attrs, op) => None,
            Self::Setlike(s) => {
                todo!()
            },
            Self::StaticMember(s) => todo!(),
            Self::Stringifier(s) => {
                todo!()
            },
        }
    }
}

impl ToTokens for Operation<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ty = &self.ty;
        let ident = self.ident.expect("regular operation has no name");
        let (args, params) = self.params.iter().fold(
            (TokenStream::new(), TokenStream::new()),
            |(mut args, mut params), (attrs, param)| {
                let ident = param.ident;
                args.append_all(quote!(#ident,));
                attrs.apply(param.into_token_stream(), &mut params);
                params.append_all(quote!(,));
                (args, params)
            }
        );
        ts.append_all(quote!(
            fn #ident(self: ::core::rc::Rc<Self>, #params) -> #ty {
                self._super().#ident(#args)
            }
        ));
    }
}

impl ToTokens for Argument<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ident = self.ident;
        let (attr, ty) = &self.ty;
        ts.append_all(quote!(#ident:));
        attr.apply(ty.into_token_stream(), ts);
    }
}

impl ToTokens for Namespace<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ident = self.ident;
        let impl_ident = ident.as_alt_ident();
        let members = self.members.iter().fold(
            TokenStream::new(),
            |mut ts, (attr, member)| {
                attr.apply(member.into_token_stream(&impl_ident), &mut ts);
                ts
            }
        );
        ts.append_all(quote!(
            pub mod #ident {
                #members
            }
        ));
    }
}

impl NamespaceMember<'_> {
    fn into_token_stream(&self, impl_ident: &Ident) -> TokenStream {
        match self {
            Self::Attribute(attr) => {
                todo!()
            },
            Self::Const(c) => {
                let ident = c.ident;
                let ty = &c.ty;
                let value = c.value;
                quote!(pub const #ident: #ty = #value;)
            },
            Self::Operation(op) => {
                let op_ident = op.ident
                    .expect("regular operation has no identifier");
                let ty = &op.ty;

                let (args, params) = op.params.iter().fold(
                    (TokenStream::new(), TokenStream::new()),
                    |(mut args, mut params), (attrs, param)| {
                        param.ident.to_tokens(&mut args);
                        attrs.apply(param.into_token_stream(), &mut params);
                        (args, params)
                    }
                );

                quote!(
                    pub fn #op_ident(#params) -> #ty {
                        #impl_ident::#op_ident(#args)
                    }
                )
            },
        }
    }
}

impl ToTokens for Typedef<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ident = self.ident;
        let (attrs, ty) = &self.ty;

        let mut ty_ts = TokenStream::new();
        attrs.apply(ty.into_token_stream(), &mut ty_ts);

        ts.append_all(quote!(pub type #ident = #ty_ts;));
    }
}

impl ToTokens for Type<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        match self {
            Self::Simple(ty) => ty.to_tokens(ts),
            Self::Variadic(ty) => ts.append_all(quote!(&[#ty])),
        }
    }
}

impl ToTokens for SimpleType<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        if self.nullable {
            let ty = &self.ty;
            ts.append_all(quote!(::core::option::Option<#ty>));
        } else {
            self.ty.to_tokens(ts);
        }
    }
}

impl ToTokens for SimpleNonnullableType<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        match self {
            Self::Any => {
                let todo = todo!();
            },
            Self::ArrayBuffer => {
                let todo = todo!();
            },
            Self::ArrayViewFloat(ty) => {
                todo!()
            },
            Self::ArrayViewInt { ty, signed } => {
                todo!()
            },
            Self::ArrayViewUint8Clamped => {
                let todo = todo!();
            },
            Self::Boolean => ts.append_all(quote!(bool)),
            Self::BigInt => {
                let todo = todo!();
            },
            Self::DataView => {
                let todo = todo!();
            },
            Self::Float { ty, restricted } => {
                if *restricted {
                    let todo = todo!();
                } else {
                    ty.to_tokens(ts);
                }
            },
            Self::FrozenArray(attrs, ty) => {
                todo!()
            },
            Self::Identifier(ident) => ts.append(ident.as_type_ident()),
            Self::Integer { ty, signed } => ty.to_tokens(ts, *signed),
            Self::Object => {
                let todo = todo!();
            },
            Self::ObservableArray(attrs, ty) => {
                todo!()
            },
            Self::Promise(ty) => {
                ts.append_all(quote!(::alloc::boxed::Box<dyn ::core::future::Future<Output = #ty>>));
            },
            Self::Record { key, value } => {
                todo!()
            },
            Self::Sequence(attrs, ty) => {
                let mut ty_ts = TokenStream::new();
                attrs.apply(ty.into_token_stream(), &mut ty_ts);
                ts.append_all(quote!(::alloc::rc::Rc<::alloc::vec::Vec<#ty_ts>>));
            },
            Self::String(ty) => ty.to_tokens(ts),
            Self::Symbol => {
                let todo = todo!();
            },
            Self::Undefined => ts.append_all(quote!(())), // The unit type
            Self::Union(types) => {
                todo!()
            },
        }
    }
}

impl IntegerType {
    fn to_tokens(&self, ts: &mut TokenStream, signed: bool) {
        if signed {
            match self {
                Self::Byte => ts.append_all(quote!(i8)),
                Self::Short => ts.append_all(quote!(i16)),
                Self::Long => ts.append_all(quote!(i32)),
                Self::LongLong => ts.append_all(quote!(i64)),
            }
        } else {
            match self {
                Self::Byte => ts.append_all(quote!(u8)),
                Self::Short => ts.append_all(quote!(u16)),
                Self::Long => ts.append_all(quote!(u32)),
                Self::LongLong => ts.append_all(quote!(u64)),
            }
        }
    }
}

impl ToTokens for FloatType {
    fn to_tokens(&self, ts: &mut TokenStream) {
        match self {
            Self::Float => ts.append_all(quote!(f32)),
            Self::Double => ts.append_all(quote!(f64)),
        }
    }
}

impl ToTokens for StringType {
    fn to_tokens(&self, ts: &mut TokenStream) {
        match self {
            Self::ByteString => ts.append_all(quote!(::alloc_rc::Rc<ByteString>)),
            Self::DomString => ts.append_all(quote!(::alloc::rc::Rc<DomString>)),
            Self::UsvString => ts.append_all(quote!(::alloc::rc::Rc<::alloc::string::String>)),
        }
    }
}

impl ToTokens for Const<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let ident = self.ident;
        let ty = &self.ty;
        let value = self.value;

        ts.append_all(quote!(pub const #ident: #ty = #value;));
    }
}

impl ToTokens for ConstValue {
    fn to_tokens(&self, ts: &mut TokenStream) {
        match *self {
            Self::Bool(b) => ts.append_all(quote!(#b)),
            Self::Float(f) => ts.append_all(quote!(#f)),
            Self::Int(i) => ts.append_all(quote!(#i)),
        }
    }
}

impl ToTokens for DefaultValue<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        match *self {
            Self::Const(c) => c.to_tokens(ts),
            Self::String(s) => {
                todo!("Convert the &str literal into a DOMString, ByteString, or USVString.");
                ts.append_all(quote!(#s))
            },
            Self::EmptySequence => todo!(),
            Self::EmptyDictionary => todo!(),
            Self::Null => ts.append_all(quote!(None)),
            Self::Undefined => panic!("attempted to represent an undefined value"),
        }
    }
}

impl ToTokens for Identifier<'_> {
    fn to_tokens(&self, ts: &mut TokenStream) {
        Ident::from(*self).to_tokens(ts);
    }
}

impl From<Identifier<'_>> for Ident {
    fn from(ident: Identifier<'_>) -> Self {
        Ident::new_raw(ident.name, Span::call_site())
    }
}

impl Identifier<'_> {
    // Returns an alternate identifier that can be used as a place for the user to fill in the rest
    // of the implementation or to place things (e.g. constants) that can't be stored in the main
    // definition (e.g. a trait).
    fn as_alt_ident(self) -> Ident {
        format_ident!("_{}", Ident::from(self))
    }

    // This is necessary because Rust requires different syntax for structs and trait object types:
    // `Foo` for structs and `dyn Foo` for traits. Type aliases accompany the type definitions to
    // accomplish this.
    fn as_type_ident(self) -> Ident {
        format_ident!("__{}", Ident::from(self))
    }
}

impl ExtendedAttributes<'_> {
    // Modifies `ts` to apply the extended attributes on `inner`.
    fn apply(&self, inner: TokenStream, ts: &mut TokenStream) {
        todo!()
    }
}
