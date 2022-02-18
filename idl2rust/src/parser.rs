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

//! This module defines the parser that powers the whole crate. It's all based directly on the
//! grammar and semantics at [https://webidl.spec.whatwg.org/].

// NOTE: Many of the parser rules in this file are rearranged from those in the standard. Since we
//       don't bother with a preliminary tokenization step, this is necessary to actually follow the
//       standard. The generic terminal symbols like `identifier` are required not to eclipse any
//       specific tokens specified in the non-terminal rules (matched in here by `keyword`, `token`,
//       and `tag`).

// NOTE: This parser does not fully validate the given IDL, but (except as outlined in the
//       documentation for the crate's root module) it does accept all IDL fragments that are valid.
//       It just fails to reject some invalid ones.

use {
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        iter,
        str::FromStr
    },
    proc_macro::{
        TokenStream,
        TokenTree,
        Ident,
        Literal,
        Span,
        quote
    },
    nom::{
        branch::alt,
        bytes::complete::*,
        character::complete::*,
        combinator::*,
        error::{Error, ErrorKind},
        multi::*,
        sequence::*,
        IResult
    }
};

// https://webidl.spec.whatwg.org/#index-prod-Definitions
pub fn parse_definitions(idl: &str) -> TokenStream {
    // This line does some lifetime-based magic that somehow convinces the borrow checker that everything's okay.
    // It seems to break if an attempt is made to use named lifetimes, and I don't understand why.
    let mut idl: &str = idl;

    let mut tts = TokenStream::new();

    let parser = Parser::new();

    // Skip past any leading whitespace and comments.
    idl = parser.eat_wsc()(idl).unwrap().0;

    // Parse all the definitions in the file.
    while eof::<_, Error<&str>>(idl).is_err() {
        match pair(
            parser.extended_attribute_list(),
            parser.definition()
        )(idl) {
            Ok((rest, (attrs, definition))) => {
                tts.extend(quote!($attrs $definition));
                idl = rest;
            },
            Err(e) => {
                let tt = TokenTree::Literal(Literal::string(e.to_string().as_str()));
                return quote!(::core::compile_error!($tt););
            }
        };
    }

    tts
}

type ParseResult<'a, T> = IResult<&'a str, T>;

struct Parser {
    current_type:                RefCell<TokenStream>,
    mod_ident:                   RefCell<TokenStream>,
    interface_parent_ident:      RefCell<TokenStream>,
    interface_consts:            RefCell<TokenStream>,
    dictionary_defaults:         RefCell<Vec<(TokenStream, TokenStream, TokenStream)>>,
    required_dictionary_members: RefCell<Vec<(TokenStream, TokenStream)>>,
    union_types:                 RefCell<Vec<(String, Vec<(String, TokenStream)>)>>,
    iter_def:                    RefCell<TokenStream>,
    method_overload_counts:      RefCell<HashMap<String, usize>>
}

impl Parser {
    fn new() -> Self {
        Self {
            current_type:                RefCell::new(TokenStream::new()),
            mod_ident:                   RefCell::new(TokenStream::new()),
            interface_parent_ident:      RefCell::new(TokenStream::new()),
            interface_consts:            RefCell::new(TokenStream::new()),
            dictionary_defaults:         RefCell::new(Vec::new()),
            required_dictionary_members: RefCell::new(Vec::new()),
            union_types:                 RefCell::new(Vec::new()),
            iter_def:                    RefCell::new(TokenStream::new()),
            method_overload_counts:      RefCell::new(HashMap::new())
        }
    }

    fn generate_union_types(&self) -> TokenStream {
        let mut union_types = TokenStream::new();
        let mut union_names = HashSet::new();
        union_types.extend(
            self.union_types.borrow_mut().drain( .. )
                .filter_map(|(union_name, mut names_types)| {
                    if union_names.contains(&union_name) {
                        return None;
                    }
                    let union_ident = TokenTree::Ident(Ident::new_raw(&union_name, Span::call_site()));
                    union_names.insert(union_name);

                    let mut inner_tts = TokenStream::new();
                    inner_tts.extend(
                        names_types.drain( .. )
                            .map(|(name, ty)| {
                                let name = TokenTree::Ident(Ident::new_raw(&name, Span::call_site()));
                                quote!($name($ty),)
                            })
                    );

                    Some(quote!(pub enum $union_ident { $inner_tts }))
                })
        );
        union_types
    }

    // https://webidl.spec.whatwg.org/#index-prod-Definition
    fn definition<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                self.callback_or_interface_or_mixin(),
                self.namespace(),
                self.partial(),
                self.dictionary(),
                self.idl_enum(),
                self.typedef(),
                self.includes_statement()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-CallbackOrInterfaceOrMixin
    fn callback_or_interface_or_mixin<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                preceded(self.keyword("callback"), self.callback_rest_or_interface()),
                preceded(self.keyword("interface"), self.interface_or_mixin())
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-InterfaceOrMixin
    fn interface_or_mixin<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                self.mixin_rest(),
                self.interface_rest()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-InterfaceRest
    fn interface_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                tuple((
                    map(
                        self.identifier_str(),
                        |ident_str| {
                            let ident_str = self.rustify_ident(ident_str);
                            self.mod_ident.replace(
                                TokenTree::Ident(Ident::new_raw(
                                    (String::from("_") + &ident_str).as_str(), Span::call_site()
                                )).into()
                            );
                            ident_str
                        }
                    ),
                    self.inheritance(),
                    delimited(
                        self.token('{'),
                        self.interface_members(),
                        pair(self.token('}'), self.token(';'))
                    )
                )), |(ident_str, inheritance, members)| {
                    let ident = TokenTree::Ident(
                        Ident::new_raw(&ident_str, Span::call_site())
                    );
                    let internal_ident = TokenTree::Ident(
                        Ident::new_raw((String::from("__") + &ident_str).as_str(), Span::mixed_site())
                    );
                    let inheritance = match inheritance {
                        Some(x) => quote!(: $x),
                        None => TokenStream::new()
                    };

                    let mod_ident = self.mod_ident.replace(TokenStream::new());
                    let union_types = self.generate_union_types();
                    let consts = self.interface_consts.replace(TokenStream::new());
                    let iter_def = self.iter_def.replace(TokenStream::new());

                    // Weirdly, this is how we can get the equivalent of `use super::*` in the
                    // generated module below.
                    let super_ident = TokenTree::Ident(Ident::new("self", Span::call_site()));

                    quote!(
                        pub trait $ident $inheritance { $members }
                        #[doc(hidden)] pub type $internal_ident = ::alloc::boxed::Box<dyn $ident>;
                        pub mod $mod_ident {
                            use $super_ident::*;
                            $union_types
                            $consts
                            $iter_def
                        }
                    )
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Partial
    fn partial<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            preceded(self.keyword("partial"), self.partial_definition())(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-PartialDefinition
    fn partial_definition<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                preceded(self.keyword("interface"), self.partial_interface_or_partial_mixin()),
                self.partial_dictionary(),
                self.namespace()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-PartialInterfaceOrPartialMixin
    fn partial_interface_or_partial_mixin<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                self.mixin_rest(),
                self.partial_interface_rest()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-PartialInterfaceRest
    fn partial_interface_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                pair(
                    self.identifier(),
                    delimited(
                        self.token('{'),
                        self.partial_interface_members(),
                        pair(self.token('}'), self.token(';'))
                    )
                ),
                |(ident, members)| quote!(pub trait $ident { $members })
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-InterfaceMembers
    fn interface_members<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            fold_many0(
                pair(self.extended_attribute_list(), self.interface_member()),
                TokenStream::new,
                |tts, (attrs, member)| quote!($tts $attrs $member)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-InterfaceMember
    fn interface_member<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                self.constructor(),
                self.partial_interface_member()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-PartialInterfaceMembers
    fn partial_interface_members<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            fold_many0(
                pair(self.extended_attribute_list(), self.partial_interface_member()),
                TokenStream::new,
                |tts, (attrs, member)| quote!($tts $attrs $member)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-PartialInterfaceMember
    fn partial_interface_member<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(
                    self.idl_const(),
                    |tts| {
                        // For object safety, a trait's constants can't be copied inline.
                        self.interface_consts.borrow_mut().extend(tts);
                        TokenStream::new()
                    }
                ),
                self.stringifier(),
                self.static_member(),
                self.iterable(),
                self.async_iterable(),
                self.read_only_member(),
                self.read_write_attribute(),
                self.read_write_maplike(),
                self.read_write_setlike(),
                self.inherit_attribute(),
                self.operation()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Inheritance
    fn inheritance<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, Option<TokenStream>> {
        |input| {
            map(
                opt(preceded(self.token(':'), self.identifier())),
                |opt_ident| {
                    match opt_ident {
                        Some(ref ident) => self.interface_parent_ident.replace(ident.clone()),
                        None => self.interface_parent_ident.replace(TokenStream::new())
                    };
                    opt_ident
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-MixinRest
    fn mixin_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                preceded(
                    self.keyword("mixin"),
                    pair(
                        self.identifier(),
                        delimited(
                            self.token('{'),
                            self.mixin_members(),
                            pair(self.token('}'), self.token(';'))
                        )
                    )
                ),
                |(ident, members)| todo!("mixin_rest")
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-MixinMembers
    fn mixin_members<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            fold_many0(
                pair(self.extended_attribute_list(), self.mixin_member()),
                TokenStream::new,
                |tts, (attrs, member)| quote!($tts $attrs $member)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-MixinMember
    fn mixin_member<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                self.idl_const(),
                self.stringifier(),
                map(
                    pair(self.optional_read_only(), self.attribute_rest()),
                    |(ro, attr)| todo!("mixin_member")
                ),
                self.regular_operation()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-IncludesStatement
    fn includes_statement<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                terminated(
                    separated_pair(
                        self.identifier(),
                        self.keyword("includes"),
                        self.identifier()
                    ),
                    self.token(';')
                ),
                |(ident1, ident2)| todo!("includes_statement")
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-CallbackRestOrInterface
    fn callback_rest_or_interface<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                self.callback_rest(),
                map(
                    pair(
                        map(
                            preceded(self.keyword("interface"), self.identifier_str()),
                            |ident_str| {
                                let ident_str = self.rustify_ident(ident_str);
                                self.mod_ident.replace(
                                    TokenTree::Ident(Ident::new_raw(
                                        (String::from("_") + &ident_str).as_str(), Span::call_site()
                                    )).into()
                                );
                                ident_str
                            }
                        ),
                        delimited(
                            self.token('{'),
                            self.callback_interface_members(),
                            pair(self.token('}'), self.token(';'))
                        )
                    ), |(ident_str, members)| {
                        let ident = TokenTree::Ident(
                            Ident::new_raw(&ident_str, Span::call_site())
                        );
                        let internal_ident = TokenTree::Ident(
                            Ident::new_raw((String::from("__") + &ident_str).as_str(), Span::mixed_site())
                        );

                        let mod_ident = self.mod_ident.replace(TokenStream::new());
                        let union_types = self.generate_union_types();
                        let consts = self.interface_consts.replace(TokenStream::new());

                        // Weirdly, this is how we can get the equivalent of `use super::*` in the
                        // generated module below.
                        let super_ident = TokenTree::Ident(Ident::new("self", Span::call_site()));

                        quote!(
                            pub trait $ident { $members }
                            #[doc(hidden)] pub type $internal_ident = ::alloc::boxed::Box<dyn $ident>;
                            pub mod $mod_ident { use $super_ident::*; $union_types $consts }
                        )
                    }
                )
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-CallbackInterfaceMembers
    fn callback_interface_members<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            fold_many0(
                pair(self.extended_attribute_list(), self.callback_interface_member()),
                TokenStream::new,
                |members, (attrs, member)| quote!($members $attrs $member)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-CallbackInterfaceMember
    fn callback_interface_member<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(
                    self.idl_const(),
                    |tts| {
                        // For object safety, a trait's constants can't be copied inline.
                        self.interface_consts.borrow_mut().extend(tts);
                        TokenStream::new()
                    }
                ),
                self.regular_operation()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Const
    fn idl_const<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                delimited(
                    self.keyword("const"),
                    tuple((
                        self.const_type(),
                        self.identifier(),
                        preceded(self.token('='), self.const_value())
                    )),
                    self.token(';')
                ),
                |(ty, ident, val)| quote!(pub const $ident: $ty = $val;)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ConstValue
    fn const_value<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                self.boolean_literal(),
                self.float_literal(),
                self.integer()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-BooleanLiteral
    fn boolean_literal<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(self.keyword("true"), |_| quote!(true)),
                map(self.keyword("false"), |_| quote!(false))
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-FloatLiteral
    fn float_literal<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                // NOTE: `-Infinity`, `Infinity`, and `NaN` are undefined when using restricted
                //       floating-point types. But we don't need to check for that, since the
                //       generated code (e.g. `Restricted<f64>::INFINITY`) will lead to
                //       a compiler error.
                map(self.keyword("-Infinity"), |_| {
                    let ty = self.current_type.borrow().clone();
                    quote!($ty::NEG_INFINITY)
                }),
                map(self.keyword("Infinity"), |_| {
                    let ty = self.current_type.borrow().clone();
                    quote!($ty::INFINITY)
                }),
                map(self.keyword("NaN"), |_| {
                    let ty = self.current_type.borrow().clone();
                    quote!($ty::NAN)
                }),
                self.decimal()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ConstType
    fn const_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(self.primitive_type(), |ty| {
                    self.current_type.replace(ty.clone());
                    ty
                }),
                map(self.identifier(), |ty| {
                    self.current_type.replace(ty.clone());
                    ty
                })
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ReadOnlyMember
    fn read_only_member<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            preceded(self.keyword("readonly"), self.read_only_member_rest())(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ReadOnlyMemberRest
    fn read_only_member_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(
                    self.attribute_rest(),
                    |(ty, ident_str)| {
                        let getter_ident = TokenTree::Ident(Ident::new_raw(&ident_str, Span::call_site()));
                        quote!(fn $getter_ident(&self) -> $ty;)
                    }
                ),
                self.maplike_rest(),
                self.setlike_rest()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ReadWriteAttribute
    fn read_write_attribute<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                self.attribute_rest(),
                |(ty, ident_str)| {
                    let getter_ident = TokenTree::Ident(Ident::new_raw(&ident_str, Span::call_site()));
                    let setter_ident = TokenTree::Ident(Ident::new_raw(
                        (String::from("_set_") + &ident_str).as_str(), Span::call_site()
                    ));
                    quote!(
                        fn $getter_ident(&self) -> $ty;
                        fn $setter_ident(&mut self, value: $ty);
                    )
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-InheritAttribute
    fn inherit_attribute<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                preceded(self.keyword("inherit"), self.attribute_rest()),
                |(ty, ident_str)| {
                    let super_ident = self.interface_parent_ident.borrow().clone();
                    let getter_ident = TokenTree::Ident(Ident::new_raw(&ident_str, Span::call_site()));
                    let setter_ident = TokenTree::Ident(Ident::new_raw(
                        (String::from("_set_") + &ident_str).as_str(), Span::call_site()
                    ));
                    quote!(
                        fn $getter_ident(&self) -> $ty {
                            (self as $super_ident).$getter_ident()
                        }
                        fn $setter_ident(&mut self, value: $ty);
                    )
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-AttributeRest
    // Returns the attribute's type and identifier separately.
    fn attribute_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, (TokenStream, String)> {
        |input| {
            delimited(
                self.keyword("attribute"),
                pair(self.type_with_extended_attributes(), self.attribute_name()),
                self.token(';')
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-AttributeName
    // https://webidl.spec.whatwg.org/#index-prod-AttributeNameKeyword
    fn attribute_name<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, String> {
        |input| {
            map(
                self.identifier_str(),
                |ident_str| self.rustify_ident(ident_str)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-OptionalReadOnly
    fn optional_read_only<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, bool> {
        |input| {
            map(
                opt(self.keyword("readonly")),
                |x| x.is_some()
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-DefaultValue
    fn default_value<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(
                    pair(self.token('['), self.token(']')),
                    |_| quote!(::core::convert::Into::into([]))
                ),
                map(
                    pair(self.token('{'), self.token('}')),
                    |_| quote!(::core::convert::Into::into(::core::default::Default::default()))
                ),
                map(
                    self.keyword("null"),
                    |_| quote!(::core::convert::Into::into(::core::option::Option::None))
                ),
                self.string(),
                self.const_value()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Operation
    fn operation<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                self.special_operation(),
                self.regular_operation()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-RegularOperation
    fn regular_operation<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                pair(
                    self.idl_type(),
                    self.operation_rest()
                ),
                |(ty, opt_op)| match opt_op {
                    Some(op) => quote!($op -> $ty;),
                    None => TokenStream::new()
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-SpecialOperation
    fn special_operation<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            preceded(
                self.special(),
                self.regular_operation()
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Special
    fn special<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, ()> {
        |input| {
            map(
                alt((
                self.keyword("getter"),
                self.keyword("setter"),
                self.keyword("deleter")
                )),
                |_| ()
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-OperationRest
    fn operation_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, Option<TokenStream>> {
        |input| {
            map(
                pair(
                    self.optional_operation_name(),
                    delimited(
                        self.token('('),
                        self.argument_list(),
                        pair(self.token(')'), self.token(';'))
                    )
                ),
                |(opt_name, (args, _))| opt_name.map(|name| quote!(fn $name(&mut self, $args)))
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-OptionalOperationName
    fn optional_operation_name<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, Option<TokenStream>> {
        |input| {
            map(
                opt(self.operation_name()),
                |opt_name| opt_name.map(|name| {
                    let mut overload_counts = self.method_overload_counts.borrow_mut();
                    if let Some(count) = overload_counts.get_mut(&name) {
                        *count += 1;
                        TokenTree::Ident(Ident::new_raw(
                            (name + "O" + &format!("{}", *count)).as_str(), Span::call_site()
                        )).into()
                    } else {
                        let ident = TokenTree::Ident(Ident::new_raw(&name, Span::call_site()));
                        overload_counts.insert(name, 0);
                        ident.into()
                    }
                })
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-OperationName
    // https://webidl.spec.whatwg.org/#index-prod-OperationNameKeyword
    fn operation_name<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, String> {
        |input| {
            map(
                self.identifier_str(),
                |ident_str| self.rustify_ident(ident_str)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ArgumentList
    // Returns a list of arguments with types and a list of types alone.
    fn argument_list<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, (TokenStream, TokenStream)> {
        |input| {
            map(
                opt(pair(self.argument(), self.arguments())),
                |x| match x {
                    Some(((arg, arg_type), (args, arg_types))) => (quote!($arg $args), quote!($arg_type, $arg_types)),
                    None => (TokenStream::new(), TokenStream::new())
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Arguments
    // Returns a list of arguments with types and a list of types alone.
    fn arguments<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, (TokenStream, TokenStream)> {
        |input| {
            fold_many0(
                preceded(self.token(','), self.argument()),
                || (TokenStream::new(), TokenStream::new()),
                |(args, arg_types), (arg, arg_type)| (quote!($args, $arg), quote!($arg_types, $arg_type))
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Argument
    // Returns the argument with its type and also its type alone.
    fn argument<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, (TokenStream, TokenStream)> {
        |input| {
            map(
                pair(self.extended_attribute_list(), self.argument_rest()),
                |(attrs, (arg, arg_type))| (quote!($attrs $arg), quote!($attrs $arg_type))
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ArgumentRest
    // Returns the argument with its type and also its type alone.
    fn argument_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, (TokenStream, TokenStream)> {
        |input| {
            alt((
                map(
                    delimited(
                        self.keyword("optional"),
                        pair(self.type_with_extended_attributes(), self.argument_name()),
                        self.default() // We ignore default values for arguments.
                    ),
                    |(ty, arg)| (quote!($arg: $ty), ty)
                ),
                map(
                    tuple((self.idl_type(), self.ellipsis(), self.argument_name())),
                    |(ty, ellipsis, arg)| {
                        let ty = if ellipsis { quote!(&[$ty]) } else { ty };
                        (quote!($arg: $ty), ty)
                    }
                )
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ArgumentName
    // https://webidl.spec.whatwg.org/#index-prod-ArgumentNameKeyword
    fn argument_name<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        // NOTE: We don't check the special cases in ArgumentNameKeyword at all, since they don't
        //       actually matter. That rule seems to be in the grammar only to assure people that
        //       almost all keywords are valid identifiers for arguments.
        self.identifier()
    }

    // https://webidl.spec.whatwg.org/#index-prod-Ellipsis
    fn ellipsis<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, bool> {
        |input| {
            map(opt(terminated(tag("..."), self.eat_wsc())), |x| x.is_some())(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Constructor
    fn constructor<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                delimited(
                    pair(self.keyword("constructor"), self.token('(')),
                    self.argument_list(),
                    pair(self.token(')'), self.token(';'))
                ),
                |(args, _)| {
                    // We have to make the identifier manually in order to avoid Rust's macro hygiene.
                    let ident = TokenTree::Ident(Ident::new_raw("_init", Span::call_site()));
                    quote!(fn $ident(&mut self, $args);)
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Stringifier
    fn stringifier<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            preceded(self.keyword("stringifier"), self.stringifier_rest())(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-StringifierRest
    fn stringifier_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(
                    pair(self.optional_read_only(), self.attribute_rest()),
                    |(ro, attr)| todo!("stringifier_rest (attribute)"),
                ),
                map(self.token(';'), |_| todo!("stringifier (;)"))
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-StaticMember
    fn static_member<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            preceded(self.keyword("static"), self.static_member_rest())(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-StaticMemberRest
    fn static_member_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(
                    pair(self.optional_read_only(), self.attribute_rest()),
                    |(ro, attr)| todo!("static_member_rest")
                ),
                self.regular_operation()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Iterable
    fn iterable<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                delimited(
                    pair(self.keyword("iterable"), self.token('<')),
                    pair(self.type_with_extended_attributes(), self.optional_type()),
                    pair(self.token('>'), self.token(';'))
                ),
                |(ty1, opt_ty2)| {
                    let item = match opt_ty2 {
                        None => ty1,
                        Some(ty2) => {
                            let item_ident = TokenTree::Ident(Ident::new_raw("KeyValue", Span::call_site()));
                            let key = TokenTree::Ident(Ident::new_raw("key", Span::call_site()));
                            let value = TokenTree::Ident(Ident::new_raw("value", Span::call_site()));
                            self.iter_def.replace(quote!(
                                pub struct $item_ident<'a> {
                                    $key: $ty1,
                                    $value: &'a mut $ty2
                                }
                            ));
                            let mod_ident = self.mod_ident.borrow().clone();
                            quote!($mod_ident::$item_ident)
                        }
                    };
                    quote!(
                        fn _iter<'a>(&mut self)
                            -> ::alloc::boxed::Box<dyn ::core::iter::Iterator::<Item = &'a mut $item>>;
                    )
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-OptionalType
    fn optional_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, Option<TokenStream>> {
        |input| {
            opt(preceded(self.token(','), self.type_with_extended_attributes()))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-AsyncIterable
    fn async_iterable<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                terminated(
                    pair(
                        delimited(
                            tuple((self.keyword("async"), self.keyword("iterable"), self.token('<'))),
                            pair(self.type_with_extended_attributes(), self.optional_type()),
                            self.token('>')
                        ),
                        self.optional_argument_list()
                    ),
                    self.token(';')
                ),
                |((ty1, opt_ty2), opt_args)| todo!("async_iterable")
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-OptionalArgumentList
    fn optional_argument_list<'a>(&'a self)
            -> impl FnMut(&'a str) -> ParseResult<'a, Option<TokenStream>> {
        |input| {
            opt(delimited(
                self.token('('),
                map(self.argument_list(), |(args, _)| args),
                self.token(')')
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ReadWriteMaplike
    fn read_write_maplike<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        self.maplike_rest()
    }

    // https://webidl.spec.whatwg.org/#index-prod-MaplikeRest
    fn maplike_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                delimited(
                    pair(self.keyword("maplike"), self.token('<')),
                    separated_pair(
                        self.type_with_extended_attributes(),
                        self.token(','),
                        self.type_with_extended_attributes()
                    ),
                    pair(self.token('>'), self.token(';'))
                ),
                |(ty1, ty2)| todo!("maplike_rest")
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ReadWriteSetlike
    fn read_write_setlike<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        self.setlike_rest()
    }

    // https://webidl.spec.whatwg.org/#index-prod-SetlikeRest
    fn setlike_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                delimited(
                    pair(self.keyword("setlike"), self.token('<')),
                    self.type_with_extended_attributes(),
                    pair(self.token('>'), self.token(';'))
                ),
                |ty| todo!("setlike_rest")
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Namespace
    fn namespace<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                pair(
                    preceded(self.keyword("namespace"), self.identifier()),
                    delimited(
                        self.token('{'),
                        self.namespace_members(),
                        pair(self.token('}'), self.token(';'))
                    )
                ),
                |(ident, members)| quote!(pub mod $ident { $members })
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-NamespaceMembers
    fn namespace_members<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            fold_many0(
                pair(self.extended_attribute_list(), self.namespace_member()),
                TokenStream::new,
                |tts, (attrs, member)| quote!($tts $attrs $member)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-NamespaceMember
    fn namespace_member<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(
                    preceded(self.keyword("readonly"), self.attribute_rest()),
                    |(ty, ident)| todo!("namespace_member")
                ),
                self.idl_const(),
                self.regular_operation()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Dictionary
    fn dictionary<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                tuple((
                    map(
                        preceded(self.keyword("dictionary"), self.identifier_str()),
                        |ident_str| {
                            let ident_str = self.rustify_ident(ident_str);
                            self.mod_ident.replace(
                                TokenTree::Ident(Ident::new_raw(
                                    (String::from("_") + &ident_str).as_str(), Span::call_site()
                                )).into()
                            );
                            ident_str
                        }
                    ),
                    self.inheritance(),
                    delimited(
                        self.token('{'),
                        self.dictionary_members(),
                        pair(self.token('}'), self.token(';'))
                    )
                )),
                |(ident_str, inheritance, members)| {
                    let ident = TokenTree::Ident(
                        Ident::new_raw(&ident_str, Span::call_site())
                    );
                    let internal_ident = TokenTree::Ident(
                        Ident::new_raw((String::from("__") + &ident_str).as_str(), Span::mixed_site())
                    );

                    let mut member_default_fns = TokenStream::new();
                    member_default_fns.extend(
                        self.dictionary_defaults.borrow().iter()
                            .map(|(member, ty, default)| {
                                let (member, ty, default) = (member.clone(), ty.clone(), default.clone());
                                quote!(pub fn $member() -> $ty { $default })
                            })
                    );

                    let mut defaults = TokenStream::new();
                    defaults.extend(
                        self.dictionary_defaults.borrow_mut().drain( .. )
                            .map(|(member, _, default)| {
                                let (member, default) = (member, default);
                                quote!($member: $default,)
                            })
                    );
                    let super_ident = TokenTree::Ident(Ident::new_raw("_super", Span::call_site()));

                    let impl_default = if !self.required_dictionary_members.borrow().is_empty() {
                        TokenStream::new()
                    } else if inheritance.is_none() {
                        quote!(impl ::core::default::Default for $ident {
                            fn default() -> Self {
                                Self {
                                    $defaults
                                }
                            }
                        })
                    } else {
                        let super_ty = inheritance.clone().unwrap();
                        quote!(impl ::core::default::Default for $ident where $super_ty: ::core::default::Default {
                            fn default() -> Self {
                                Self {
                                    $super_ident: ::core::default::Default::default(),
                                    $defaults
                                }
                            }
                        })
                    };
                    self.required_dictionary_members.borrow_mut().clear();

                    let inheritance = match inheritance {
                        Some(ty) => quote!(pub $super_ident: $ty,),
                        None => TokenStream::new()
                    };

                    let mod_ident = self.mod_ident.replace(TokenStream::new());
                    let union_types = self.generate_union_types();
                    let iter_def = self.iter_def.replace(TokenStream::new());

                    // Weirdly, this is how we can get the equivalent of `use super::*` in the
                    // generated module below.
                    let super_ident = TokenTree::Ident(Ident::new("self", Span::call_site()));

                    quote!(
                        pub struct $ident { $inheritance $members }
                        #[doc(hidden)] pub type $internal_ident = $ident;
                        impl $ident { $member_default_fns }
                        pub mod $mod_ident {
                            use $super_ident::*;
                            $union_types
                            $iter_def
                        }
                        $impl_default
                    )
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-DictionaryMembers
    fn dictionary_members<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            fold_many0(
                self.dictionary_member(),
                TokenStream::new,
                |members, member| quote!($members $member)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-DictionaryMember
    fn dictionary_member<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                pair(
                    self.extended_attribute_list(),
                    self.dictionary_member_rest()
                ),
                |(attrs, member)| quote!($attrs $member)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-DictionaryMemberRest
    fn dictionary_member_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(
                    pair(
                        preceded(self.keyword("required"), self.type_with_extended_attributes()),
                        terminated(self.identifier(), self.token(';'))
                    ),
                    |(ty, ident)| {
                        self.required_dictionary_members.borrow_mut().push((ident.clone(), ty.clone()));
                        quote!(pub $ident: $ty,)
                    }
                ),
                map(
                    tuple((
                        self.idl_type(),
                        self.identifier(),
                        terminated(self.default(), self.token(';'))
                    )),
                    |(ty, ident, default)| {
                        let (ty, default) = match default {
                            Some(default) => (ty, quote!($default)),
                            None => (
                                quote!(::core::option::Option::<$ty>),
                                quote!(::core::option::Option::None)
                            )
                        };
                        self.dictionary_defaults.borrow_mut().push((ident.clone(), ty.clone(), default));
                        quote!(pub $ident: $ty,)
                    }
                ),
                self.idl_type()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-PartialDictionary
    fn partial_dictionary<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                pair(
                    preceded(self.keyword("dictionary"), self.identifier()),
                    delimited(
                        self.token('{'),
                        self.dictionary_members(),
                        pair(self.token('}'), self.token(';'))
                    )
                ),
                |(ident, members)| quote!(pub struct $ident { $members })
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Default
    fn default<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, Option<TokenStream>> {
        |input| {
            opt(preceded(self.token('='), self.default_value()))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Enum
    fn idl_enum<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                pair(
                    preceded(self.keyword("enum"), self.identifier()),
                    delimited(
                        self.token('{'),
                        self.enum_value_list(),
                        pair(self.token('}'), self.token(';'))
                    )
                ),
                |(ident, (values, len))| {
                    let len = TokenTree::Literal(Literal::usize_unsuffixed(len));
                    quote!(pub static $ident: [&str; $len] = [$values])
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-EnumValueList
    // https://webidl.spec.whatwg.org/#index-prod-EnumValueListComma
    // https://webidl.spec.whatwg.org/#index-prod-EnumValueListString
    // Returns the token stream and the number of strings inside it.
    fn enum_value_list<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, (TokenStream, usize)> {
        |input| {
            map(
                terminated(
                    pair(
                        fold_many0(
                            terminated(self.string(), self.token(',')),
                            || (TokenStream::new(), 0),
                            |(tts, len), s| (quote!($tts $s,), len + 1)
                        ),
                        self.string()
                    ),
                    opt(self.token(','))
                ),
                |((tts, len), s)| (quote!($tts $s), len + 1)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-CallbackRest
    fn callback_rest<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                tuple((
                    self.identifier_str(),
                    preceded(self.token('='), self.idl_type()),
                    delimited(
                        self.token('('),
                        self.argument_list(),
                        pair(self.token(')'), self.token(';'))
                    )
                )),
                |(ident_str, ty, (_, arg_types))| {
                    let ident_str = self.rustify_ident(ident_str);
                    let ident = TokenTree::Ident(Ident::new_raw(&ident_str, Span::call_site()));
                    let internal_ident = TokenTree::Ident(Ident::new_raw(
                        (String::from("__") + &ident_str).as_str(), Span::mixed_site()
                    ));
                    quote!(
                        pub type $ident = ::alloc::boxed::Box<dyn ::core::ops::FnMut($arg_types) -> $ty>;
                        #[doc(hidden)] pub type $internal_ident = $ident;
                    )
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Typedef
    fn typedef<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                delimited(
                    self.keyword("typedef"),
                    pair(self.type_with_extended_attributes(), self.identifier_str()),
                    self.token(';')
                ),
                |(ty, ident_str)| {
                    let ident_str = self.rustify_ident(ident_str);
                    let ident = TokenTree::Ident(Ident::new_raw(&ident_str, Span::call_site()));
                    let internal_ident = TokenTree::Ident(Ident::new_raw(
                        (String::from("__") + &ident_str).as_str(), Span::mixed_site()
                    ));
                    quote!(
                        pub type $ident = $ty;
                        #[doc(hidden)] pub type $internal_ident = $ident;
                    )
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Type
    fn idl_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                alt((
                    map(
                        pair(self.union_type(), self.null()),
                        |((_, ty), null)| if null { quote!(::core::option::Option::<$ty>) } else { ty }
                    ),
                    self.single_type()
                )),
                |ty| {
                    self.current_type.replace(ty.clone());
                    ty
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-TypeWithExtendedAttributes
    fn type_with_extended_attributes<'a>(&'a self)
            -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                pair(
                    self.extended_attribute_list(),
                    self.idl_type()
                ),
                |(attrs, ty)| if attrs.is_empty() {
                    ty
                } else {
                    todo!("type_with_extended_attributes")
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-SingleType
    fn single_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(self.keyword("any"), |_| quote!(::alloc::boxed::Box<dyn ::core::any::Any>)),
                self.promise_type(),
                self.distinguishable_type()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-UnionType
    // Returns the type name in IDL and the type tokens in Rust.
    fn union_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, (String, TokenStream)> {
        |input| {
            delimited(
                self.token('('),
                map(
                    pair(self.union_member_type(), self.union_member_types()),
                    |((name, ty), mut names_tys)| {
                        let union_name = String::from("_Union_") +
                            iter::once(name.clone()).chain(
                                names_tys.iter().map(|(name, _)| String::from("_or_") + name)
                            ).collect::<String>().as_str();
                        let ident = TokenTree::Ident(Ident::new_raw(&union_name, Span::call_site()));

                        self.union_types.borrow_mut().push(
                            (
                                union_name.clone(),
                                iter::once((name, ty)).chain(names_tys.drain( .. )).collect()
                            )
                        );

                        let mod_ident = self.mod_ident.borrow().clone();

                        (union_name, quote!($mod_ident::$ident))
                    }
                ),
                self.token(')')
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-UnionMemberType
    // Returns the type name in IDL and the type tokens in Rust.
    fn union_member_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, (String, TokenStream)> {
        |input| {
            alt((
                map(
                    pair(self.extended_attribute_list(), consumed(self.distinguishable_type())),
                    |(attrs, (ty_name, ty))| (
                        String::from(ty_name.trim()),
                        quote!($attrs $ty)
                    )
                ),
                map(
                    pair(self.union_type(), self.null()),
                    |((ty_name, ty_tts), null)| {
                        let ty = if null {
                            quote!(::core::option::Option::<$ty_tts>)
                        } else {
                            ty_tts
                        };
                        let ty_name = String::from("_") + &ty_name + "_";
                        (ty_name, ty)
                    }
                )
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-UnionMemberTypes
    // Modified to use `fold_many1` instead of `fold_many0`
    fn union_member_types<'a>(&'a self)
            -> impl FnMut(&'a str) -> ParseResult<'a, Vec<(String, TokenStream)>> {
        |input| {
            fold_many1(
                preceded(self.keyword("or"), self.union_member_type()),
                Vec::new,
                |mut names_tys, (name, ty)| {
                    names_tys.push((name, ty));
                    names_tys
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-DistinguishableType
    fn distinguishable_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(
                    pair(self.primitive_type(), self.null()),
                    |(ty, null)| if null { quote!(::core::option::Option::<$ty>) } else { ty }
                ),
                map(
                    pair(self.string_type(), self.null()),
                    |(ty, null)| if null { quote!(::core::option::Option::<$ty>) } else { ty }
                ),
                map(
                    pair(
                        preceded(
                            self.keyword("sequence"),
                            delimited(
                                self.token('<'),
                                self.type_with_extended_attributes(),
                                self.token('>')
                            )
                        ),
                        self.null()
                    ),
                    |(ty, null)| {
                        let ty = if null { quote!(::core::option::Option::<$ty>) } else { ty };
                        quote!(::alloc::vec::Vec::<$ty>)
                    }
                ),
                map(
                    preceded(self.keyword("object"), self.null()),
                    |null| if null { quote!(::core::option::Option::<Object>) } else { quote!(Object) }
                ),
                map(
                    preceded(self.keyword("symbol"), self.null()),
                    |null| todo!("distinguishable_type (symbol)")
                ),
                map(
                    pair(self.buffer_related_type(), self.null()),
                    |(ty, null)| if null { quote!(::core::option::Option::<$ty>) } else { ty }
                ),
                map(
                    pair(
                        preceded(
                            self.keyword("FrozenArray"),
                            delimited(
                                self.token('<'),
                                self.type_with_extended_attributes(),
                                self.token('>')
                            )
                        ),
                        self.null()
                    ),
                    |(ty, null)| {
                        let ty = if null { quote!(::core::option::Option::<$ty>) } else { ty };
                        quote!(&[$ty])
                    }
                ),
                map(
                    pair(
                        preceded(
                            self.keyword("ObservableArray"),
                            delimited(
                                self.token('<'),
                                self.type_with_extended_attributes(),
                                self.token('>')
                            )
                        ),
                        self.null()
                    ),
                    |(ty, null)| todo!("distinguishable_type (ObservableArray)")
                ),
                map(
                    pair(self.record_type(), self.null()),
                    |(ty, null)| if null { quote!(::core::option::Option::<$ty>) } else { ty }
                ),
                map(
                    pair(self.identifier_str(), self.null()),
                    |(s, null)| {
                        // We prepend underscores to make dictionaries and interfaces work with
                        // the same syntax, since Rust requires `dyn` at the beginning of a trait
                        // object's type (and it would be a DST anyway). The type alias with a
                        // leading underscore is defined at the same time as each struct and trait.
                        let ident_str = self.rustify_ident(s);
                        let ty = TokenTree::Ident(
                            Ident::new_raw((String::from("__") + &ident_str).as_str(), Span::mixed_site())
                        );
                        if null { quote!(::core::option::Option::<$ty>) } else { ty.into() }
                    }
                )
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-PrimitiveType
    fn primitive_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                self.unsigned_integer_type(),
                self.unrestricted_float_type(),
                map(self.keyword("undefined"), |_| quote!(())),
                map(self.keyword("boolean"), |_| quote!(bool)),
                map(self.keyword("byte"), |_| quote!(u8)),
                map(self.keyword("octet"), |_| quote!(u8)),
                map(self.keyword("bigint"), |_| quote!(BigInt))
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-UnrestrictedFloatType
    fn unrestricted_float_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                preceded(self.keyword("unrestricted"), self.float_type()),
                map(
                    self.float_type(),
                    |tts| {
                        let restricted = TokenTree::Ident(Ident::new_raw("Restricted", Span::call_site()));
                        quote!($restricted::<$tts>)
                    }
                )
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-FloatType
    fn float_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(self.keyword("float"), |_| quote!(f32)),
                map(self.keyword("double"), |_| quote!(f64))
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-UnsignedIntegerType
    // https://webidl.spec.whatwg.org/#index-prod-IntegerType
    // https://webidl.spec.whatwg.org/#index-prod-OptionalLong
    fn unsigned_integer_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(preceded(self.keyword("unsigned"), pair(self.keyword("long"), self.keyword("long"))), |_| quote!(u64)),
                map(preceded(self.keyword("unsigned"), self.keyword("long")), |_| quote!(u32)),
                map(preceded(self.keyword("unsigned"), self.keyword("short")), |_| quote!(u16)),
                map(pair(self.keyword("long"), self.keyword("long")), |_| quote!(i64)),
                map(self.keyword("long"), |_| quote!(i32)),
                map(self.keyword("short"), |_| quote!(i16))
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-StringType
    fn string_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(self.keyword("ByteString"), |_| quote!(::alloc::vec::Vec<u8>)),
                map(self.keyword("DOMString"), |_| quote!(::alloc::vec::Vec<u16>)),
                map(self.keyword("USVString"), |_| quote!(::alloc::string::String))
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-PromiseType
    fn promise_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                preceded(
                    self.keyword("Promise"),
                    delimited(
                        self.token('<'),
                        self.idl_type(),
                        self.token('>')
                    )
                ),
                |ty| quote!(impl ::core::future::Future<$ty>)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-RecordType
    fn record_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                preceded(
                    self.keyword("record"),
                    delimited(
                        self.token('<'),
                        separated_pair(
                            self.string_type(),
                            self.token(','),
                            self.type_with_extended_attributes()
                        ),
                        self.token('>')
                    )
                ),
                |(s_type, attr_type)| todo!("record_type")
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Null
    fn null<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, bool> {
        |input| {
            map(
                opt(self.token('?')),
                |x| x.is_some()
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-BufferRelatedType
    fn buffer_related_type<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(self.keyword("ArrayBuffer"), |_| todo!("buffer_related_type (ArrayBuffer)")),
                map(self.keyword("DataView"), |_| todo!("buffer_related_type (DataView)")),
                map(self.keyword("Int8Array"), |_| quote!(::alloc::vec::Vec<i8>)),
                map(self.keyword("Int16Array"), |_| quote!(::alloc::vec::Vec<i16>)),
                map(self.keyword("Int32Array"), |_| quote!(::alloc::vec::Vec<i32>)),
                map(self.keyword("Uint8Array"), |_| quote!(::alloc::vec::Vec<u8>)),
                map(self.keyword("Uint16Array"), |_| quote!(::alloc::vec::Vec<u16>)),
                map(self.keyword("Uint32Array"), |_| quote!(::alloc::vec::Vec<u32>)),
                map(self.keyword("Uint8ClampedArray"), |_| todo!("buffer_related_type (Uint8ClampedArray)")),
                map(self.keyword("BigInt64Array"), |_| quote!(::alloc::vec::Vec<i64>)),
                map(self.keyword("BigUint64Array"), |_| quote!(::alloc::vec::Vec<u64>)),
                map(self.keyword("Float32Array"), |_| quote!(::alloc::vec::Vec<f32>)),
                map(self.keyword("Float64Array"), |_| quote!(::alloc::vec::Vec<f64>))
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ExtendedAttributeList
    fn extended_attribute_list<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                opt(delimited(
                    self.token('['),
                    pair(self.extended_attribute(), self.extended_attributes()),
                    self.token(']')
                )),
                |x| match x {
                    Some((attr, attrs)) => quote!($attr $attrs),
                    None => TokenStream::new()
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ExtendedAttributes
    fn extended_attributes<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            fold_many0(
                preceded(self.token(','), self.extended_attribute()),
                TokenStream::new,
                |tts, attrs| quote!($tts $attrs)
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ExtendedAttribute
    // https://webidl.spec.whatwg.org/#index-prod-ExtendedAttributeRest
    fn extended_attribute<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                recognize(fold_many0(
                    alt((
                        delimited(self.token('('), self.extended_attribute_inner(), self.token(')')),
                        delimited(self.token('['), self.extended_attribute_inner(), self.token(']')),
                        delimited(self.token('{'), self.extended_attribute_inner(), self.token('}')),
                        self.other()
                    )),
                    || (),
                    |(), _| ()
                )),
                |s| {
                    println!("\x1b[93mwarning\x1b[0m: found unrecognized extended attribute [{}]", s);
                    TokenStream::new()
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-ExtendedAttributeInner
    fn extended_attribute_inner<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<&'a str> {
        |input| {
            // Because of Rust's lack of true support for curried functions and because of the need to keep
            // `self` around to keep track of state, a recursive solution won't work here. The iterative
            // solution isn't as pretty, but it works.

            let mut rest = input;
            let mut len = 0;
            let mut closing_delimiters_stack = Vec::new();

            loop {
                if let Ok((r, (s, c))) = consumed(alt((self.token('('), self.token('['), self.token('{'))))(rest) {
                    closing_delimiters_stack.push(match c {
                        '(' => ')',
                        '[' => ']',
                        '{' => '}',
                        _ => unreachable!()
                    });
                    rest = r;
                    len += s.len();
                    continue;
                }
                if let Some(&delim) = closing_delimiters_stack.last() {
                    if let Ok((r, s)) = recognize(self.token(delim))(rest) {
                        closing_delimiters_stack.pop();
                        rest = r;
                        len += s.len();
                        continue;
                    }
                }
                if let Ok((r, s)) = recognize(self.other_or_comma())(rest) {
                    rest = r;
                    len += s.len();
                    continue;
                }
                break;
            }

            if closing_delimiters_stack.is_empty() {
                let (val, remainder) = input.split_at(len);
                assert_eq!(rest as *const _, remainder as *const _);
                Ok((rest, val))
            } else {
                Err(nom::Err::Error(Error::new(input, ErrorKind::Char)))
            }
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-Other
    fn other<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<&'a str> {
        // NOTE: This is significantly cut down from what the spec says, since the identities of
        //       these tokens are completely irrelevant at this point. We could just reduce it to
        //       finding all characters that satisfy /[^()\[\]{},]/, except that the spec's grammar
        //       forbids matching things like `42foo` outside of a string.
        |input| {
            alt((
                recognize(self.string()),
                recognize(self.integer()),
                recognize(self.decimal()),
                recognize(self.identifier()),
                recognize(self.other_terminal())
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#index-prod-OtherOrComma
    fn other_or_comma<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<&'a str> {
        |input| {
            alt((
                recognize(self.token(',')),
                self.other()
            ))(input)
        }
    }

    // https://webidl.spec.whatwg.org/#prod-integer
    fn integer<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                terminated(
                    pair(
                        opt(char('-')),
                        alt((
                            self.uint_dec(),
                            self.uint_hex(),
                            self.uint_oct()
                        ))
                    ),
                    self.eat_wsc()
                ),
                |(sign, magnitude)| {
                    let tt = TokenTree::Literal(Literal::u128_unsuffixed(magnitude));
                    match sign {
                        Some(_) => quote!(-$tt),
                        None => quote!($tt)
                    }
                }
            )(input)
        }
    }

    fn uint_dec<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, u128> {
        |input| {
            map(
                recognize(pair(
                    verify(satisfy(|c| c.is_digit(10)), |c| *c != '0'),
                    digit0
                )),
                |int_str| u128::from_str_radix(int_str, 10)
                    .expect("parser error when reading decimal integer")
            )(input)
        }
    }

    fn uint_hex<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, u128> {
        |input| {
            map(
                preceded(
                    tag_no_case("0x"),
                    hex_digit1
                ),
                |int_str| u128::from_str_radix(int_str, 16)
                    .expect("parser error when reading hexadecimal integer")
            )(input)
        }
    }

    fn uint_oct<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, u128> {
        |input| {
            map(
                recognize(preceded(
                    char('0'),
                    oct_digit0
                )),
                |int_str| u128::from_str_radix(int_str, 8)
                    .expect("parser error when reading octal integer")
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#prod-decimal
    fn decimal<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                terminated(
                    pair(
                        opt(char('-')),
                        alt((
                            recognize(tuple((
                                digit0,
                                char('.'),
                                digit1,
                                opt(tuple((
                                    one_of("Ee"),
                                    opt(one_of("+-")),
                                    digit1
                                )))
                            ))),
                            recognize(tuple((
                                digit1,
                                opt(char('.')),
                                digit0,
                                one_of("Ee"),
                                opt(one_of("+-")),
                                digit1
                            )))
                        ))
                    ),
                    self.eat_wsc()
                ),
                |(sign, magnitude)| {
                    let tt = TokenTree::Literal(Literal::f64_unsuffixed(
                        f64::from_str(magnitude).expect("parser error when reading floating-point number")
                    ));
                    match sign {
                        Some(_) => quote!(-$tt),
                        None => quote!($tt)
                    }
                }
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#prod-identifier
    fn identifier<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                self.identifier_str(),
                |s| TokenTree::Ident(Ident::new_raw(self.rustify_ident(s).as_str(), Span::call_site())).into()
            )(input)
        }
    }

    fn identifier_str<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, &str> {
        |input| {
            map(
                terminated(
                    recognize(tuple((
                        opt(one_of("_-")),
                        satisfy(|c| c.is_ascii_alphabetic()),
                        many0_count(satisfy(|c| c.is_ascii_alphanumeric() || c == '_'))
                    ))),
                    self.eat_wsc()
                ),
                |ident| {
                    // "For all of these constructs, the identifier is the value of the identifier token with any
                    // leading U+005F LOW LINE ("_") character (underscore) removed."
                    let mut chars = ident.chars();
                    if let Some('_') = chars.next() {
                        chars.as_str()
                    } else {
                        ident
                    }
                }
            )(input)
        }
    }

    // Converts an IDL identifier into a Rust identifier. This means converting a leading hyphen into two underscores
    // (which is invalid for the beginning of an IDL identifier) and changing camel case to snake case.
    fn rustify_ident(&self, idl_ident: &str) -> String {
        assert!(!idl_ident.is_empty());

        let (mut rust_ident, rest) = if idl_ident.starts_with('-') {
            (String::from("___"), &idl_ident[1 ..])
        } else {
            (String::new(), idl_ident)
        };

        if self.is_screaming_snake_case(rest) {
            // SCREAMING_SNAKE_CASE is fine.
            // TODO: This is usually used for enumerations. Should we convert to UpperCamelCase and generate enums?
            return rust_ident + rest;
        }

        // An out-of-bounds access is impossible here because the only character we could have removed by now is a hyphen.
        // Every IDL identifier must contain at least one letter.
        if rest.chars().next().expect("parser error when reading identifier").is_ascii_uppercase() {
            // Assume this is UpperCamelCase. That is also fine.
            rust_ident + rest
        } else {
            // Otherwise, assume it's lowerCamelCase. This should be converted to snake_case.
            for c in rest.chars() {
                if c.is_ascii_uppercase() {
                    rust_ident.push('_');
                    rust_ident.push(c.to_ascii_lowercase());
                } else {
                    rust_ident.push(c);
                }
            }
            rust_ident
        }
    }

    fn is_screaming_snake_case(&self, s: &str) -> bool {
        s.chars().position(|c| c.is_ascii_lowercase()).is_none()
    }

    // Matches the given keyword while ensuring that it's not matching just the prefix of a longer
    // identifier. (For instance, "longWord" should never be seen as keyword "long" followed by identifier "Word".)
    fn keyword<'a>(&'a self, word: &'a str) -> impl FnMut(&'a str) -> ParseResult<&'a str> {
        move |input| {
            verify(self.identifier_str(), move |s: &str| s == word)(input)
        }
    }

    // https://webidl.spec.whatwg.org/#prod-string
    fn string<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                terminated(
                    delimited(
                        char('"'),
                        take_until("\""),
                        char('"')
                    ),
                    self.eat_wsc()
                ),
                |s| TokenTree::Literal(Literal::string(s)).into()
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#prod-whitespace
    fn whitespace<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, ()> {
        |input| {
            map(multispace1, |_| ())(input)
        }
    }

    // https://webidl.spec.whatwg.org/#prod-comment
    fn comment<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, ()> {
        |input| {
            map(
                alt((
                    preceded(
                        tag("//"),
                        not_line_ending
                    ),
                    delimited(
                        tag("/*"),
                        take_until("*/"),
                        tag("*/")
                    )
                )),
                |_| ()
            )(input)
        }
    }

    // A convenience function to skip past whitespace and comments
    fn eat_wsc<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, ()> {
        |input| {
            fold_many0(
                alt((self.whitespace(), self.comment())), || (), |(), ()| ()
            )(input)
        }
    }

    // https://webidl.spec.whatwg.org/#prod-other
    fn other_terminal<'a>(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, char> {
        |input| {
            terminated(
                satisfy(|c|
                    // NOTE: Every symbol explicitly mentioned as a token in the grammar should really
                    //       be excluded here, but these are the only ones that matter for our purposes.
                    c != '\t' && c != '\n' && c != '\r' && c != ' ' && !c.is_ascii_alphanumeric()
                        && c != '(' && c != ')' && c != '[' && c != ']' && c != '{' && c != '}' && c != ','
                ),
                self.eat_wsc()
            )(input)
        }
    }

    // Matches a given character and discards any whitespace and comments after it.
    fn token<'a>(&'a self, c: char) -> impl FnMut(&'a str) -> ParseResult<'a, char> {
        move |input| {
            terminated(char(c), self.eat_wsc())(input)
        }
    }
}
