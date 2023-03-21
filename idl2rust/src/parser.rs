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

//! This module defines the parser that powers the whole crate. It's all based directly on the
//! grammar and semantics at [https://webidl.spec.whatwg.org/].

// NOTE: Many of the parser rules in this file are rearranged from those in the standard. Since we
//       don't bother with a preliminary tokenization step, this is necessary to actually follow the
//       standard. The generic terminal symbols like `identifier` are required not to eclipse any
//       specific tokens specified in the non-terminal rules (matched in here by `token` and `tag`).

// NOTE: This parser does not fully validate the given IDL, but (except as outlined in the
//       documentation for the crate's root module) it does accept all IDL fragments that are valid.
//       It just fails to reject some invalid ones.

use {
    std::str::FromStr,
    nom::{
        IResult,
        branch::*,
        bytes::complete::*,
        character::complete::*,
        combinator::*,
        multi::*,
        sequence::*,
    },
    crate::ast::*,
};

pub fn parse(input: &str) -> Result<Definitions, nom::Err<nom::error::Error<&str>>> {
    let (_, ast) = all_consuming(
        terminated(definitions, ws_and_comments)
    )(&input)?;

    Ok(ast)
}

// https://webidl.spec.whatwg.org/#index-prod-Definitions
fn definitions(input: &str) -> IResult<&str, Definitions> {
    map(
        many0(pair(extended_attribute_list, definition)),
        |defs| Definitions { defs }
    )(input)
}
/* FIXME: Remove this.
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
        match preceded(
            parser.extended_attribute_list(&parser.definition_attrs),
            parser.definition()
        )(idl) {
            Ok((rest, definition)) => {
                assert!(parser.definition_attrs.borrow().is_empty(), "didn't use the extended attributes");
                tts.extend(quote!($definition));
                idl = rest;
            },
            Err(e) => {
                let tt = TokenTree::Literal(Literal::string(e.to_string().as_str()));
                return quote!(::core::compile_error!($tt););
            }
        };
    }

    tts
}*/

/* FIXME: Remove this.
type ParseResult<'a, T> = IResult<&'a str, T>;*/

/* FIXME: Remove this.
struct Parser<'a> {
    current_type:                RefCell<TokenStream>,
    mod_ident:                   RefCell<TokenStream>,
    interface_parent_ident:      RefCell<TokenStream>,
    interface_consts:            RefCell<TokenStream>,
    dictionary_defaults:         RefCell<Vec<(TokenStream, TokenStream, TokenStream)>>,
    required_dictionary_members: RefCell<Vec<(TokenStream, TokenStream)>>,
    union_types:                 RefCell<Vec<(String, Vec<(String, TokenStream)>)>>,
    iter_def:                    RefCell<TokenStream>,
    method_overload_counts:      RefCell<HashMap<String, usize>>,
    definition_attrs:            RefCell<Vec<ExtendedAttribute<'a>>>,
    member_attrs:                RefCell<Vec<ExtendedAttribute<'a>>>,
    type_attrs:                  RefCell<Vec<ExtendedAttribute<'a>>>
}

impl<'a> Parser<'a> {
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
            method_overload_counts:      RefCell::new(HashMap::new()),
            definition_attrs:            RefCell::new(Vec::new()),
            member_attrs:                RefCell::new(Vec::new()),
            type_attrs:                  RefCell::new(Vec::new())
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
    }*/

// https://webidl.spec.whatwg.org/#index-prod-Definition
fn definition(input: &str) -> IResult<&str, Definition> {
    alt((
        callback_or_interface_or_mixin,
        map(namespace, |ns| Definition::Namespace(ns)),
        // Note: We don't support partial definitions.
        map(dictionary, |d| Definition::Dictionary(d)),
        map(enum_, |e| Definition::Enum(e)),
        map(typedef, |td| Definition::Typedef(td)),
        // Note: We don't support includes statements.
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-CallbackOrInterfaceOrMixin
fn callback_or_interface_or_mixin(input: &str) -> IResult<&str, Definition> {
    preceded(
        ws_and_comments,
        alt((
            preceded(tag("callback"), callback_rest_or_interface),
            map(
                preceded(tag("interface"), interface_or_mixin),
                |i| Definition::Interface(i),
            ),
        )),
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-InterfaceOrMixin
fn interface_or_mixin(input: &str) -> IResult<&str, Interface> {
    interface_rest(input)
    // Note: We don't support mixins.
}

// https://webidl.spec.whatwg.org/#index-prod-InterfaceRest
fn interface_rest(input: &str) -> IResult<&str, Interface> {
    map(
        tuple((
            identifier,
            inheritance,
            delimited(token("{"), interface_members, pair(token("}"), token(";")))
        )),
        |(ident, inheritance, members)| Interface {
            ident,
            inheritance,
            members,
        },
    )(input)
}
/* FIXME: Remove this.
    fn interface_rest(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
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
                    #[cfg(feature = "intertrait")]
                    let inheritance = match inheritance {
                        Some(x) => quote!(: $x + ::intertrait::CastFrom),
                        None => quote!(: ::intertrait::CastFrom)
                    };
                    #[cfg(not(feature = "intertrait"))]
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

                    let mut tts = quote!(
                        pub trait $ident $inheritance { $members }
                    );

                    for attr in self.definition_attrs.borrow_mut().drain( .. ) {
                        match attr {
                            ExtendedAttribute::Ident("Exposed", _) |
                            ExtendedAttribute::IdentList("Exposed", _) |
                            ExtendedAttribute::Wildcard("Exposed") => {},
                            ExtendedAttribute::NoArgs("LegacyUnenumerableNamedProperties") => {},
                            ref attr => {
                                let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                tts = quote!($macro_ident ! ($attr $tts););
                            }
                        };
                    }

                    quote!(
                        $tts
                        #[doc(hidden)] pub type $internal_ident = ::alloc::rc::Rc<dyn $ident>;
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
    }*/

// https://webidl.spec.whatwg.org/#index-prod-InterfaceMembers
fn interface_members(input: &str) -> IResult<&str, Vec<(ExtendedAttributes, InterfaceMember)>> {
    many0(pair(extended_attribute_list, interface_member))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-InterfaceMember
fn interface_member(input: &str) -> IResult<&str, InterfaceMember> {
    alt((
        map(constructor, |op| InterfaceMember::Constructor(op)),
        partial_interface_member,
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-PartialInterfaceMember
fn partial_interface_member(input: &str) -> IResult<&str, InterfaceMember> {
    alt((
        map(const_, |c| InterfaceMember::Const(c)),
        map(operation, |o| InterfaceMember::Operation(ExtendedAttributes::new(), o)),
        map(stringifier, |s| InterfaceMember::Stringifier(s)),
        map(static_member, |m| InterfaceMember::StaticMember(m)),
        map(iterable, |i| InterfaceMember::Iterable(i)),
        map(async_iterable, |i| InterfaceMember::Iterable(i)),
        read_only_member,
        map(read_write_attribute, |a| InterfaceMember::Attribute(a)),
        map(read_write_maplike, |m| InterfaceMember::Maplike(m)),
        map(read_write_setlike, |s| InterfaceMember::Setlike(s)),
        map(inherit_attribute, |a| InterfaceMember::Attribute(a)),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Inheritance
fn inheritance(input: &str) -> IResult<&str, Option<Identifier>> {
    opt(identifier)(input)
}

// https://webidl.spec.whatwg.org/#index-prod-CallbackRestOrInterface
fn callback_rest_or_interface(input: &str) -> IResult<&str, Definition> {
    alt((
        map(callback_rest, |cb| Definition::CallbackFunction(cb)),
        map(
            preceded(
                token("interface"),
                pair(
                    identifier,
                    delimited(
                        token("{"),
                        callback_interface_members,
                        pair(token("}"), token(";"))
                    ),
                ),
            ),
            |(ident, members)| Definition::CallbackInterface(
                CallbackInterface {
                    ident,
                    members,
                }
            ),
        ),
    ))(input)
}
/* FIXME: Remove this.
    fn callback_rest_or_interface(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
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

                        let mut tts = quote!(
                            pub trait $ident { $members }
                        );

                        for attr in self.definition_attrs.borrow_mut().drain( .. ) {
                            match attr {
                                ExtendedAttribute::Ident("Exposed", _) |
                                ExtendedAttribute::IdentList("Exposed", _) |
                                ExtendedAttribute::Wildcard("Exposed") => {},
                                ref attr => {
                                    let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                    tts = quote!($macro_ident ! ($attr $tts););
                                }
                            };
                        }

                        quote!(
                            $tts
                            #[doc(hidden)] pub type $internal_ident = ::alloc::rc::Rc<dyn $ident>;
                            pub mod $mod_ident { use $super_ident::*; $union_types $consts }
                        )
                    }
                )
            ))(input)
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-CallbackInterfaceMembers
fn callback_interface_members(input: &str) -> IResult<&str, Vec<(ExtendedAttributes, CallbackInterfaceMember)>> {
    many0(pair(extended_attribute_list, callback_interface_member))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-CallbackInterfaceMember
fn callback_interface_member(input: &str) -> IResult<&str, CallbackInterfaceMember> {
    alt((
        map(const_, |c| CallbackInterfaceMember::Const(c)),
        map(regular_operation, |op| CallbackInterfaceMember::Operation(op)),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Const
fn const_(input: &str) -> IResult<&str, Const> {
    map(
        delimited(
            token("const"),
            tuple((
                const_type,
                identifier,
                preceded(token("="), const_value),
            )),
            token(";"),
        ),
        |(ty, ident, value)| Const { ty, ident, value },
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ConstValue
fn const_value(input: &str) -> IResult<&str, ConstValue> {
    alt((
        map(boolean_literal, |b| ConstValue::Bool(b)),
        map(float_literal, |f| ConstValue::Float(f)),
        map(integer, |i| ConstValue::Int(i)),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-BooleanLiteral
fn boolean_literal(input: &str) -> IResult<&str, bool> {
    alt((
        value(true, token("true")),
        value(false, token("false")),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-FloatLiteral
fn float_literal(input: &str) -> IResult<&str, f64> {
    alt((
        value(f64::NEG_INFINITY, token("-Infinity")),
        value(f64::INFINITY, token("Infinity")),
        value(f64::NAN, token("NaN")),
        decimal,
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ConstType
fn const_type(input: &str) -> IResult<&str, SimpleNonnullableType> {
    alt((
        primitive_type,
        map(
            identifier,
            |ident| SimpleNonnullableType::Identifier(ident),
        ),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ReadOnlyMember
fn read_only_member(input: &str) -> IResult<&str, InterfaceMember> {
    preceded(token("readonly"), read_only_member_rest)(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ReadOnlyMemberRest
fn read_only_member_rest(input: &str) -> IResult<&str, InterfaceMember> {
    alt((
        map(
            attribute_rest,
            |rest| InterfaceMember::Attribute(
                Attribute::new(AttributeTag::ReadOnly, rest)
            ),
        ),
        map(
            maplike_rest,
            |(from, to)| InterfaceMember::Maplike(
                Maplike {
                    readonly: true,
                    from,
                    to,
                }
            ),
        ),
        map(
            setlike_rest,
            |ty| InterfaceMember::Setlike(
                Setlike {
                    readonly: true,
                    ty,
                }
            ),
        ),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ReadWriteAttribute
fn read_write_attribute(input: &str) -> IResult<&str, Attribute> {
    map(
        attribute_rest,
        |rest| Attribute::new(AttributeTag::None, rest),
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-InheritAttribute
fn inherit_attribute(input: &str) -> IResult<&str, Attribute> {
    map(
        preceded(token("inherit"), attribute_rest),
        |rest| Attribute::new(AttributeTag::Inherit, rest),
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-AttributeRest
fn attribute_rest(input: &str) -> IResult<&str, AttributeRest> {
    map(
        delimited(
            token("attribute"),
            pair(type_with_extended_attributes, attribute_name),
            token(";"),
        ),
        |((attrs, ty), name)| AttributeRest {
            ty: (attrs, Type::Simple(ty)),
            name
        },
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-AttributeName
// https://webidl.spec.whatwg.org/#index-prod-AttributeNameKeyword
fn attribute_name(input: &str) -> IResult<&str, Identifier> {
    // Note: We don't have special cases for keywords here because the `identifier` function covers them.
    identifier(input)
}

// https://webidl.spec.whatwg.org/#index-prod-OptionalReadOnly
fn optional_read_only(input: &str) -> IResult<&str, bool> {
    map(opt(token("readonly")), |o| o.is_some())(input)
}

// https://webidl.spec.whatwg.org/#index-prod-DefaultValue
fn default_value(input: &str) -> IResult<&str, DefaultValue> {
    alt((
        value(DefaultValue::EmptySequence, pair(token("["), token("]"))),
        value(DefaultValue::EmptyDictionary, pair(token("{"), token("}"))),
        value(DefaultValue::Null, token("null")),
        value(DefaultValue::Undefined, token("undefined")),
        map(string, |s| DefaultValue::String(s)),
        map(const_value, |c| DefaultValue::Const(c)),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Operation
fn operation(input: &str) -> IResult<&str, Operation> {
    alt((regular_operation, special_operation))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-RegularOperation
fn regular_operation(input: &str) -> IResult<&str, Operation> {
    map(
        pair(type_, operation_rest),
        |(ty, rest)| Operation::new(ty, rest)
    )(input)
}
/* FIXME: Remove this.
    fn regular_operation(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            map(
                pair(
                    self.idl_type(),
                    self.operation_rest()
                ),
                |(ty, opt_op)| match opt_op {
                    Some(op) => {
                        let mut tts = quote!($op -> $ty;);

                        for attr in self.member_attrs.borrow_mut().drain( .. ) {
                            match attr {
                                ExtendedAttribute::Ident("Exposed", _) |
                                ExtendedAttribute::IdentList("Exposed", _) |
                                ExtendedAttribute::Wildcard("Exposed") => {},
                                ExtendedAttribute::NoArgs("NewObject") => {},
                                ExtendedAttribute::NoArgs("Unscopable") => {},
                                ExtendedAttribute::NoArgs("LegacyUnforgeable") => {},
                                ref attr => {
                                    let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                    tts = quote!($macro_ident ! ($attr $tts););
                                }
                            };
                        }
                        tts
                    },
                    None => TokenStream::new()
                }
            )(input)
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-SpecialOperation
fn special_operation(input: &str) -> IResult<&str, Operation> {
    preceded(special, regular_operation)(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Special
fn special(input: &str) -> IResult<&str, ()> {
    alt((token("getter"), token("setter"), token("deleter")))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-OperationRest
fn operation_rest(input: &str) -> IResult<&str, OperationRest> {
    map(
        pair(
            optional_operation_name,
            delimited(
                token("("),
                argument_list,
                pair(token(")"), token(";"))
            ),
        ),
        |(ident, params)| OperationRest { ident, params }
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-OptionalOperationName
fn optional_operation_name(input: &str) -> IResult<&str, Option<Identifier>> {
    opt(operation_name)(input)
}
/* FIXME: Remove this. It implements overload name mangling.
    fn optional_operation_name(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, Option<TokenStream>> {
        |input| {
            map(
                opt(self.operation_name()),
                |opt_name| opt_name.map(|name| {
                    let mut overload_counts = self.method_overload_counts.borrow_mut();
                    if let Some(count) = overload_counts.get_mut(&name) {
                        *count += 1;
                        TokenTree::Ident(Ident::new_raw(
                            format!("_O{}_{}", *count, name).as_str(), Span::call_site()
                        )).into()
                    } else {
                        let ident = TokenTree::Ident(Ident::new_raw(&name, Span::call_site()));
                        overload_counts.insert(name, 0);
                        ident.into()
                    }
                })
            )(input)
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-OperationName
// https://webidl.spec.whatwg.org/#index-prod-OperationNameKeyword
fn operation_name(input: &str) -> IResult<&str, Identifier> {
    // Note: We don't have a special case for the keyword `includes` because the `identifier` function covers it.
    identifier(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ArgumentList
// https://webidl.spec.whatwg.org/#index-prod-Arguments
fn argument_list(input: &str) -> IResult<&str, Vec<(ExtendedAttributes, Argument)>> {
    separated_list0(token(","), argument)(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Argument
fn argument(input: &str) -> IResult<&str, (ExtendedAttributes, Argument)> {
    pair(extended_attribute_list, argument_rest)(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ArgumentRest
fn argument_rest(input: &str) -> IResult<&str, Argument> {
    alt((
        map(
            preceded(
                token("optional"),
                tuple((type_with_extended_attributes, argument_name, default)),
            ),
            |((attrs, ty), ident, default)| Argument {
                ty: (attrs, Type::Simple(ty)),
                ident,
                default,
            },
        ),
        map(
            tuple((type_, ellipsis, argument_name)),
            |(ty, ellipsis, ident)| Argument {
                ty: (ExtendedAttributes::new(), if ellipsis { Type::Variadic(ty) } else { Type::Simple(ty) }),
                ident,
                default: None,
            },
        ),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ArgumentName
// https://webidl.spec.whatwg.org/#index-prod-ArgumentNameKeyword
fn argument_name(input: &str) -> IResult<&str, Identifier> {
    identifier(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Ellipsis
fn ellipsis(input: &str) -> IResult<&str, bool> {
    map(opt(token("...")), |o| o.is_some())(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Constructor
fn constructor(input: &str) -> IResult<&str, Vec<(ExtendedAttributes, Argument)>> {
    delimited(
        pair(token("constructor"), token("(")),
        argument_list,
        pair(token(")"), token(";")),
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Stringifier
fn stringifier(input: &str) -> IResult<&str, Stringifier> {
    preceded(token("stringifier"), stringifier_rest)(input)
}

// https://webidl.spec.whatwg.org/#index-prod-StringifierRest
fn stringifier_rest(input: &str) -> IResult<&str, Stringifier> {
    alt((
        map(
            pair(optional_read_only, attribute_rest),
            |(readonly, rest)| Stringifier::Attribute(
                Attribute::new(
                    if readonly { AttributeTag::ReadOnly } else { AttributeTag::None },
                    rest
                )
            ),
        ),
        value(Stringifier::Simple, token(";")),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-StaticMember
fn static_member(input: &str) -> IResult<&str, StaticMember> {
    preceded(token("static"), static_member_rest)(input)
}

// https://webidl.spec.whatwg.org/#index-prod-StaticMemberRest
fn static_member_rest(input: &str) -> IResult<&str, StaticMember> {
    alt((
        map(
            pair(optional_read_only, attribute_rest),
            |(readonly, rest)| StaticMember::Attribute(
                Attribute::new(
                    if readonly { AttributeTag::ReadOnly } else { AttributeTag::None },
                    rest,
                )
            ),
        ),
        map(
            regular_operation,
            |op| StaticMember::Operation(op)
        ),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Iterable
fn iterable(input: &str) -> IResult<&str, Iterable> {
    map(
        delimited(
           pair(token("iterable"), token("<")),
           pair(type_with_extended_attributes, optional_type),
           pair(token(">"), token(";")), 
        ),
        |(ty1, ty2)| match  ty2 {
            Some(ty2) => Iterable::Sync { key: Some(ty1), value: ty2 },
            None => Iterable::Sync { key: None, value: ty1 },
        },
    )(input)
}
/* FIXME: Remove this.
    fn iterable(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
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
                            quote!($mod_ident::$item_ident<'_>)
                        }
                    };

                    let ident = TokenTree::Ident(Ident::new_raw("_iter", Span::call_site()));
                    let mut tts = quote!(
                        fn $ident(&mut self)
                            -> ::alloc::boxed::Box<dyn ::core::iter::Iterator::<Item = &mut $item> + '_>;
                    );

                    for attr in self.member_attrs.borrow_mut().drain( .. ) {
                        match attr {
                            ExtendedAttribute::Ident("Exposed", _) |
                            ExtendedAttribute::IdentList("Exposed", _) |
                            ExtendedAttribute::Wildcard("Exposed") => {},
                            ref attr => {
                                let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                tts = quote!($macro_ident ! ($attr $tts););
                            }
                        };
                    }
                    tts
                }
            )(input)
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-OptionalType
fn optional_type(input: &str) -> IResult<&str, Option<(ExtendedAttributes, SimpleType)>> {
    opt(preceded(token(","), type_with_extended_attributes))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-AsyncIterable
fn async_iterable(input: &str) -> IResult<&str, Iterable> {
    map(
        terminated(
            pair(
                delimited(
                    tuple((token("async"), token("iterable"), token("<"))),
                    pair(type_with_extended_attributes, optional_type),
                    token(">"),
                ),
                optional_argument_list,
            ),
            token(";"),
        ),
        |((ty1, ty2), args)| {
            let args = args.unwrap_or_else(Vec::new);
            match ty2 {
                Some(ty2) => Iterable::Async {
                    key: Some(ty1),
                    value: ty2,
                    args,
                },
                None => Iterable::Async {
                    key: None,
                    value: ty1,
                    args,
                },
            }
        },
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-OptionalArgumentList
fn optional_argument_list(input: &str) -> IResult<&str, Option<Vec<(ExtendedAttributes, Argument)>>> {
    opt(delimited(token("("), argument_list, token(")")))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ReadWriteMaplike
fn read_write_maplike(input: &str) -> IResult<&str, Maplike> {
    map(maplike_rest, |(from, to)| Maplike { readonly: false, from, to })(input)
}

// https://webidl.spec.whatwg.org/#index-prod-MaplikeRest
fn maplike_rest(input: &str) -> IResult<&str, ((ExtendedAttributes, SimpleType), (ExtendedAttributes, SimpleType))> {
    delimited(
        pair(token("maplike"), token("<")),
        separated_pair(type_with_extended_attributes, token(","), type_with_extended_attributes),
        pair(token(">"), token(";")),
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ReadWriteSetlike
fn read_write_setlike(input: &str) -> IResult<&str, Setlike> {
    map(
        setlike_rest,
        |ty| Setlike { readonly: false, ty },
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-SetlikeRest
fn setlike_rest(input: &str) -> IResult<&str, (ExtendedAttributes, SimpleType)> {
    delimited(
        pair(token("setlike"), token("<")),
        type_with_extended_attributes,
        pair(token(">"), token(";")),
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Namespace
fn namespace(input: &str) -> IResult<&str, Namespace> {
    map(
        delimited(
            token("namespace"),
            pair(
                identifier,
                delimited(token("{"), namespace_members, token("}")),
            ),
            token(";"),
        ),
        |(ident, members)| Namespace { ident, members },
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-NamespaceMembers
fn namespace_members(input: &str) -> IResult<&str, Vec<(ExtendedAttributes, NamespaceMember)>> {
    many0(pair(extended_attribute_list, namespace_member))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-NamespaceMember
fn namespace_member(input: &str) -> IResult<&str, NamespaceMember> {
    alt((
        map(
            preceded(token("readonly"), attribute_rest),
            |rest| NamespaceMember::Attribute(
                Attribute::new(AttributeTag::ReadOnly, rest)
            ),
        ),
        map(
            const_,
            |c| NamespaceMember::Const(c),
        ),
        map(
            regular_operation,
            |op| NamespaceMember::Operation(op),
        ),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Dictionary
fn dictionary(input: &str) -> IResult<&str, Dictionary> {
    map(
        delimited(
            token("dictionary"),
            tuple((
                identifier,
                inheritance,
                delimited(token("{"), dictionary_members, token("}")),
            )),
            token(";"),
        ),
        |(ident, inheritance, members)| Dictionary { ident, inheritance, members },
    )(input)
}
/* FIXME: Remove this.
    fn dictionary(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
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

                    let mut tts = quote!(
                        pub struct $ident { $inheritance $members }
                        #[doc(hidden)] pub type $internal_ident = ::alloc::rc::Rc<$ident>;
                        impl $ident { $member_default_fns }
                        pub mod $mod_ident {
                            use $super_ident::*;
                            $union_types
                            $iter_def
                        }
                        $impl_default
                    );

                    for attr in self.definition_attrs.borrow_mut().drain( .. ) {
                        match attr {
                            ref attr => {
                                let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                tts = quote!($macro_ident ! ($attr $tts););
                            }
                        };
                    }
                    tts
                }
            )(input)
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-DictionaryMembers
fn dictionary_members(input: &str) -> IResult<&str, Vec<(ExtendedAttributes, DictionaryMember)>> {
    many0(dictionary_member)(input)
}

// https://webidl.spec.whatwg.org/#index-prod-DictionaryMember
fn dictionary_member(input: &str) -> IResult<&str, (ExtendedAttributes, DictionaryMember)> {
    pair(extended_attribute_list, dictionary_member_rest)(input)
}

// https://webidl.spec.whatwg.org/#index-prod-DictionaryMemberRest
fn dictionary_member_rest(input: &str) -> IResult<&str, DictionaryMember> {
    alt((
        map(
            delimited(
                token("required"),
                pair(type_with_extended_attributes, identifier),
                token(";"),
            ),
            |(ty, ident)| DictionaryMember {
                ty,
                ident,
                default: None,
            },
        ),
        map(
            terminated(
                tuple((type_, identifier, default)),
                token(";"),
            ),
            |(ty, ident, default)| DictionaryMember {
                ty: (ExtendedAttributes::new(), ty),
                ident,
                default: Some(default.unwrap_or(DefaultValue::Undefined)),
            },
        ),
    ))(input)
}
/* FIXME: Remove this.
    fn dictionary_member_rest(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
        |input| {
            alt((
                map(
                    pair(
                        preceded(self.keyword("required"), self.type_with_extended_attributes()),
                        terminated(self.identifier(), self.token(';'))
                    ),
                    |(ty, ident)| {
                        self.required_dictionary_members.borrow_mut().push((ident.clone(), ty.clone()));

                        let mut tts = quote!(pub $ident: $ty,);

                        for attr in self.member_attrs.borrow_mut().drain( .. ) {
                            match attr {
                                ref attr => {
                                    let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                    tts = quote!($macro_ident ! ($attr $tts););
                                }
                            };
                        }
                        tts
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
                        let mut tts = quote!(pub $ident: $ty,);

                        for attr in self.member_attrs.borrow_mut().drain( .. ) {
                            match attr {
                                ref attr => {
                                    let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                    tts = quote!($macro_ident ! ($attr $tts););
                                }
                            };
                        }
                        tts
                    }
                ),
                self.idl_type()
            ))(input)
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-Default
fn default(input: &str) -> IResult<&str, Option<DefaultValue>> {
    opt(preceded(token("="), default_value))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Enum
fn enum_(input: &str) -> IResult<&str, Enum> {
    map(
        delimited(
            token("enum"),
            pair(
                identifier,
                delimited(token("{"), enum_value_list, token("}")),
            ),
            token(";"),
        ),
        |(ident, values)| Enum { ident, values },
    )(input)
}
/* FIXME: Remove this.
    fn idl_enum(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
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

                    let mut tts = quote!(pub static $ident: [&str; $len] = [$values]);

                    for attr in self.member_attrs.borrow_mut().drain( .. ) {
                        match attr {
                            ref attr => {
                                let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                tts = quote!($macro_ident ! ($attr $tts););
                            }
                        };
                    }
                    tts
                }
            )(input)
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-EnumValueList
// https://webidl.spec.whatwg.org/#index-prod-EnumValueListComma
// https://webidl.spec.whatwg.org/#index-prod-EnumValueListString
fn enum_value_list(input: &str) -> IResult<&str, Vec<&str>> {
    terminated(
        separated_list1(token(","), string),
        opt(token(",")),
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-CallbackRest
fn callback_rest(input: &str) -> IResult<&str, CallbackFunction> {
    map(
        separated_pair(
            identifier,
            token("="),
            pair(
                type_,
                delimited(
                    token("("),
                    argument_list,
                    pair(token(")"), token(";")),
                )
            ),
        ),
        |(ident, (ty, params))| CallbackFunction {
            ident,
            ty,
            params,
        },
    )(input)
}
/* FIXME: Remove this.
    fn callback_rest(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
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

                    let mut tts = quote!(
                        pub type $ident = ::alloc::rc::Rc<dyn ::core::ops::FnMut($arg_types) -> $ty>;
                        #[doc(hidden)] pub type $internal_ident = $ident;
                    );

                    for attr in self.definition_attrs.borrow_mut().drain( .. ) {
                        match attr {
                            ExtendedAttribute::NoArgs("LegacyTreatNonObjectAsNull") => {},
                            ref attr => {
                                let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                tts = quote!($macro_ident ! ($attr $tts););
                            }
                        };
                    }
                    tts
                }
            )(input)
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-Typedef
fn typedef(input: &str) -> IResult<&str, Typedef> {
    map(
        delimited(
            token("typedef"),
            pair(type_with_extended_attributes, identifier),
            token(";"),
        ),
        |(ty, ident)| Typedef { ty, ident },
    )(input)
}
/* FIXME: Remove this.
    fn typedef(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, TokenStream> {
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

                    let mut tts = quote!(
                        pub type $ident = $ty;
                        #[doc(hidden)] pub type $internal_ident = $ident;
                    );

                    for attr in self.definition_attrs.borrow_mut().drain( .. ) {
                        match attr {
                            ref attr => {
                                let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                tts = quote!($macro_ident ! ($attr $tts););
                            }
                        };
                    }
                    tts
                }
            )(input)
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-Type
fn type_(input: &str) -> IResult<&str, SimpleType> {
    alt((
        single_type,
        map(
            pair(union_type, null),
            |(ty, nullable)| SimpleType { nullable, ..ty },
        )
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-TypeWithExtendedAttributes
fn type_with_extended_attributes(input: &str) -> IResult<&str, (ExtendedAttributes, SimpleType)> {
    pair(extended_attribute_list, type_)(input)
}

// https://webidl.spec.whatwg.org/#index-prod-SingleType
fn single_type(input: &str) -> IResult<&str, SimpleType> {
    alt((
        value(SimpleType { ty: SimpleNonnullableType::Any, nullable: false, }, token("any")),
        map(promise_type, |ty| SimpleType { ty, nullable: false }),
        distinguishable_type,
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-UnionType
// https://webidl.spec.whatwg.org/#index-prod-UnionMemberTypes
fn union_type(input: &str) -> IResult<&str, SimpleType> {
    map(
        delimited(
            token("("),
            verify(
                separated_list1(token("or"), union_member_type),
                |types: &Vec<_>| types.len() > 1,
            ),
            token(")"),
        ),
        |types| SimpleType {
            ty: SimpleNonnullableType::Union(types),
            nullable: false,
        },
    )(input)
}
/* FIXME: Remove this.
    // Returns the type's name and the tokens needed to refer to it.
    fn union_type(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, (String, TokenStream)> {
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
                        let mut tts = quote!($mod_ident::$ident);

                        for attr in self.type_attrs.borrow_mut().drain( .. ) {
                            match attr {
                                ref attr => {
                                    let macro_ident = Self::custom_extended_attribute_macro_ident(attr);
                                    tts = quote!($macro_ident ! ($attr $tts););
                                }
                            };
                        }
                        (union_name, tts)
                    }
                ),
                self.token(')')
            )(input)
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-UnionMemberType
fn union_member_type(input: &str) -> IResult<&str, (ExtendedAttributes, SimpleType)> {
    alt((
        pair(extended_attribute_list, distinguishable_type),
        map(
            pair(union_type, null),
            |(ty, nullable)| (ExtendedAttributes::new(), SimpleType { nullable, ..ty }),
        ),
    ))(input)
}
/* FIXME: Remove this.
    // Returns the type name in IDL and the type tokens in Rust.
    fn union_member_type(&'a self) -> impl FnMut(&'a str) -> ParseResult<'a, (String, TokenStream)> {
        |input| {
            // A union can be an annotated type that contains annotated types. Make sure the two sets
            // of extended attributes don't interfere with each other.
            let outer_type_attrs = self.type_attrs.replace(Vec::new());

            let result = alt((
                map(
                    preceded(self.extended_attribute_list(&self.type_attrs), consumed(self.distinguishable_type())),
                    |(ty_name, ty)| {
                        assert!(self.type_attrs.borrow().is_empty(), "didn't use the extended attributes");
                        (String::from(ty_name.trim()), ty)
                    }
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
            ))(input);

            self.type_attrs.replace(outer_type_attrs);
            result
        }
    }*/

// https://webidl.spec.whatwg.org/#index-prod-DistinguishableType
fn distinguishable_type(input: &str) -> IResult<&str, SimpleType> {
    alt((
        map(
            preceded(
                token("sequence"),
                pair(
                    delimited(token("<"), type_with_extended_attributes, token(">")),
                    null,
                ),
            ),
            |((attrs, ty), nullable)| SimpleType {
                ty: SimpleNonnullableType::Sequence(attrs, Box::new(ty)),
                nullable,
            },
        ),
        map(
            preceded(token("object"), null),
            |nullable| SimpleType {
                ty: SimpleNonnullableType::Object,
                nullable,
            },
        ),
        map(
            preceded(token("symbol"), null),
            |nullable| SimpleType {
                ty: SimpleNonnullableType::Symbol,
                nullable,
            },
        ),
        map(
            preceded(
                token("FrozenArray"),
                pair(
                    delimited(token("<"), type_with_extended_attributes, token(">")),
                    null,
                ),
            ),
            |((attrs, ty), nullable)| SimpleType {
                ty: SimpleNonnullableType::FrozenArray(attrs, Box::new(ty)),
                nullable,
            },
        ),
        map(
            preceded(
                token("ObservableArray"),
                pair(
                    delimited(token("<"), type_with_extended_attributes, token(">")),
                    null,
                ),
            ),
            |((attrs, ty), nullable)| SimpleType {
                ty: SimpleNonnullableType::ObservableArray(attrs, Box::new(ty)),
                nullable,
            },
        ),
        map(
            preceded(token("undefined"), null),
            |nullable| SimpleType {
                ty: SimpleNonnullableType::Undefined,
                nullable,
            },
        ),
        map(
            pair(primitive_type, null),
            |(ty, nullable)| SimpleType { ty, nullable },
        ),
        map(
            pair(string_type, null),
            |(ty, nullable)| SimpleType {
                ty: SimpleNonnullableType::String(ty),
                nullable
            },
        ),
        map(
            pair(buffer_related_type, null),
            |(ty, nullable)| SimpleType { ty, nullable },
        ),
        map(
            pair(record_type, null),
            |(ty, nullable)| SimpleType { ty, nullable },
        ),
        map(
            pair(identifier, null),
            |(ident, nullable)| SimpleType {
                ty: SimpleNonnullableType::Identifier(ident),
                nullable,
            },
        )
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-PrimitiveType
fn primitive_type(input: &str) -> IResult<&str, SimpleNonnullableType> {
    alt((
        value(SimpleNonnullableType::Boolean, token("boolean")),
        value(
            SimpleNonnullableType::Integer {
                ty: IntegerType::Byte,
                signed: true,
            },
            token("byte"),
        ),
        value(
            SimpleNonnullableType::Integer {
                ty: IntegerType::Byte,
                signed: false,
            },
            token("octet"),
        ),
        value(SimpleNonnullableType::BigInt, token("bigint")),
        unsigned_integer_type,
        unrestricted_float_type,
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-UnrestrictedFloatType
fn unrestricted_float_type(input: &str) -> IResult<&str, SimpleNonnullableType> {
    map(
        pair(opt(token("unrestricted")), float_type),
        |(unrestricted, ty)| SimpleNonnullableType::Float {
            ty,
            restricted: unrestricted.is_none(),
        },
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-FloatType
fn float_type(input: &str) -> IResult<&str, FloatType> {
    alt((
        value(FloatType::Float, token("float")),
        value(FloatType::Double, token("double")),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-UnsignedIntegerType
fn unsigned_integer_type(input: &str) -> IResult<&str, SimpleNonnullableType> {
    map(
        pair(opt(token("unsigned")), integer_type),
        |(unsigned, ty)| SimpleNonnullableType::Integer {
            ty,
            signed: unsigned.is_none(),
        },
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-IntegerType
fn integer_type(input: &str) -> IResult<&str, IntegerType> {
    alt((
        value(IntegerType::Short, token("short")),
        map(
            preceded(token("long"), optional_long),
            |longer| if longer { IntegerType::LongLong } else { IntegerType::Long },
        ),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-OptionalLong
fn optional_long(input: &str) -> IResult<&str, bool> {
    map(opt(token("long")), |o| o.is_some())(input)
}

// https://webidl.spec.whatwg.org/#index-prod-StringType
fn string_type(input: &str) -> IResult<&str, StringType> {
    alt((
        value(StringType::ByteString, token("ByteString")),
        value(StringType::DomString, token("DOMString")),
        value(StringType::UsvString, token("USVString")),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-PromiseType
fn promise_type(input: &str) -> IResult<&str, SimpleNonnullableType> {
    map(
        delimited(
            pair(token("Promise"), token("<")),
            type_,
            token(">"),
        ),
        |ty| SimpleNonnullableType::Promise(Box::new(ty)),
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-RecordType
fn record_type(input: &str) -> IResult<&str, SimpleNonnullableType> {
    map(
        delimited(
            pair(token("record"), token("<")),
            separated_pair(string_type, token(","), type_with_extended_attributes),
            token(">"),
        ),
        |(key, (attrs, value))| SimpleNonnullableType::Record {
            key,
            value: (attrs, Box::new(value)),
        },
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Null
fn null(input: &str) -> IResult<&str, bool> {
    map(opt(token("?")), |o| o.is_some())(input)
}

// https://webidl.spec.whatwg.org/#index-prod-BufferRelatedType
fn buffer_related_type(input: &str) -> IResult<&str, SimpleNonnullableType> {
    alt((
        value(SimpleNonnullableType::ArrayBuffer, token("ArrayBuffer")),
        value(SimpleNonnullableType::DataView, token("DataView")),
        value(
            SimpleNonnullableType::ArrayViewInt {
                ty: IntegerType::Byte,
                signed: true,
            },
            token("Int8Array")
        ),
        value(
            SimpleNonnullableType::ArrayViewInt {
                ty: IntegerType::Short,
                signed: true,
            },
            token("Int16Array")
        ),
        value(
            SimpleNonnullableType::ArrayViewInt {
                ty: IntegerType::Long,
                signed: true,
            },
            token("Int32Array")
        ),
        value(
            SimpleNonnullableType::ArrayViewInt {
                ty: IntegerType::LongLong,
                signed: true,
            },
            token("BigInt64Array")
        ),
        value(
            SimpleNonnullableType::ArrayViewInt {
                ty: IntegerType::Byte,
                signed: false,
            },
            token("Uint8Array")
        ),
        value(
            SimpleNonnullableType::ArrayViewInt {
                ty: IntegerType::Short,
                signed: false,
            },
            token("Uint16Array")
        ),
        value(
            SimpleNonnullableType::ArrayViewInt {
                ty: IntegerType::Long,
                signed: false,
            },
            token("Uint32Array")
        ),
        value(
            SimpleNonnullableType::ArrayViewInt {
                ty: IntegerType::LongLong,
                signed: false,
            },
            token("BigUint64Array")
        ),
        value(SimpleNonnullableType::ArrayViewUint8Clamped, token("Uint8ClampedArray")),
        value(SimpleNonnullableType::ArrayViewFloat(FloatType::Float), token("Float32Array")),
        value(SimpleNonnullableType::ArrayViewFloat(FloatType::Double), token("Float64Array")),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ExtendedAttributeList
// https://webidl.spec.whatwg.org/#index-prod-ExtendedAttributes
fn extended_attribute_list(input: &str) -> IResult<&str, ExtendedAttributes> {
    map(
        opt(delimited(token("["), separated_list1(token(","), extended_attribute), token("]"))),
        |attrs| ExtendedAttributes { attrs: attrs.unwrap_or_else(Vec::new) },
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ExtendedAttribute
// https://webidl.spec.whatwg.org/#index-prod-ExtendedAttributeRest
fn extended_attribute(input: &str) -> IResult<&str, &str> {
    recognize(
        many1_count(
            alt((
                delimited(token("("), extended_attribute_inner, token(")")),
                delimited(token("["), extended_attribute_inner, token("]")),
                delimited(token("{"), extended_attribute_inner, token("}")),
                other_nonterminal,
            )),
        ),
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-ExtendedAttributeInner
fn extended_attribute_inner(input: &str) -> IResult<&str, &str> {
    recognize(
        many0_count(
            alt((
                delimited(token("("), extended_attribute_inner, token(")")),
                delimited(token("["), extended_attribute_inner, token("]")),
                delimited(token("{"), extended_attribute_inner, token("}")),
                other_or_comma,
            )),
        ),
    )(input)
}

// https://webidl.spec.whatwg.org/#index-prod-Other
fn other_nonterminal(input: &str) -> IResult<&str, &str> {
    alt((
        // Note: All the specific tokens are omitted from this list because, with how we generate tokens, the more general
        //       rules already cover those cases.
        recognize(integer),
        recognize(decimal),
        recognize(identifier),
        recognize(string),
        recognize(other),
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-OtherOrComma
fn other_or_comma(input: &str) -> IResult<&str, &str> {
    alt((
        recognize(token(",")),
        other_nonterminal,
    ))(input)
}

// https://webidl.spec.whatwg.org/#index-prod-IdentifierList
// https://webidl.spec.whatwg.org/#index-prod-Identifiers
fn identifier_list(input: &str) -> IResult<&str, Vec<Identifier>> {
    separated_list1(token(","), identifier)(input)
}

// https://webidl.spec.whatwg.org/#prod-ExtendedAttributeNoArgs
fn extended_attribute_no_args(input: &str) -> IResult<&str, ExtendedAttribute> {
    map(identifier, |ident| ExtendedAttribute::NoArgs(ident))(input)
}

// https://webidl.spec.whatwg.org/#prod-ExtendedAttributeArgList
fn extended_attribute_arg_list(input: &str) -> IResult<&str, ExtendedAttribute> {
    map(
        pair(
            identifier,
            delimited(token("("), argument_list, token(")")),
        ),
        |(ident, args)| ExtendedAttribute::ArgList(ident, args),
    )(input)
}

// https://webidl.spec.whatwg.org/#prod-ExtendedAttributeIdent
fn extended_attribute_ident(input: &str) -> IResult<&str, ExtendedAttribute> {
    map(
        separated_pair(identifier, token("="), identifier),
        |(lhs, rhs)| ExtendedAttribute::Ident(lhs, rhs),
    )(input)
}

// https://webidl.spec.whatwg.org/#prod-ExtendedAttributeWildcard
fn extended_attribute_wildcard(input: &str) -> IResult<&str, ExtendedAttribute> {
    map(
        terminated(identifier, pair(token("="), token("*"))),
        |ident| ExtendedAttribute::Wildcard(ident),
    )(input)
}

// https://webidl.spec.whatwg.org/#prod-ExtendedAttributeIdentList
fn extended_attribute_ident_list(input: &str) -> IResult<&str, ExtendedAttribute> {
    map(
        separated_pair(
            identifier,
            token("="),
            delimited(token("("), identifier_list, token(")")),
        ),
        |(lhs, idents)| ExtendedAttribute::IdentList(lhs, idents),
    )(input)
}

// https://webidl.spec.whatwg.org/#prod-ExtendedAttributeNamedArgList
fn extended_attribute_named_arg_list(input: &str) -> IResult<&str, ExtendedAttribute> {
    map(
        separated_pair(
            identifier,
            token("="),
            pair(
                identifier,
                delimited(token("("), argument_list, token(")")),
            ),
        ),
        |(lhs, (rhs, args))| ExtendedAttribute::NamedArgList(lhs, rhs, args),
    )(input)
}

fn token(s: &str) -> impl FnMut(&str) -> IResult<&str, ()> + '_ {
    move |input| value(
        (),
        preceded(
            pair(
                ws_and_comments,
                // Make sure this isn't just the start of an identifier.
                // Note: This doesn't work if the identifier starts with an underscore, but that's okay because no keywords
                //       are defined that start with underscores.
                not(verify(identifier, |ident| ident.name.len() > s.len()))
            ),
            tag(s),
        ),
    )(input)
}

// -----------------------------------------------
// Named terminal symbols (should always be checked last)
// -----------------------------------------------

// https://webidl.spec.whatwg.org/#prod-integer
fn integer(input: &str) -> IResult<&str, i128> {
    map(
        preceded(
            ws_and_comments,
            pair(
                opt(char('-')),
                alt((
                    map(
                        preceded(tag_no_case("0x"), hex_digit1),
                        |s| (s, 16),
                    ),
                    map(
                        preceded(char('0'), oct_digit1),
                        |s| (s, 8),
                    ),
                    map(
                        digit1,
                        |s| (s, 10),
                    ),
                )),
            ),
        ),
        |(sign, (digits, radix))| {
            let s = match sign {
                Some(_) => format!("-{digits}"),
                None => String::from(digits),
            };
            i128::from_str_radix(&s, radix)
                .expect("internal parser error")
        },
    )(input)
}

// https://webidl.spec.whatwg.org/#prod-decimal
fn decimal(input: &str) -> IResult<&str, f64> {
    map(
        preceded(
            ws_and_comments,
            recognize(pair(
                opt(char('-')),
                alt((
                    recognize(pair(
                        alt((
                            recognize(separated_pair(
                                digit1,
                                char('.'),
                                digit0,
                            )),
                            recognize(preceded(
                                char('.'),
                                digit1,
                            )),
                        )),
                        opt(tuple((
                            one_of("Ee"),
                            opt(one_of("+-")),
                            digit1,
                        ))),
                    )),
                    recognize(tuple((
                        digit1,
                        one_of("Ee"),
                        opt(one_of("+-")),
                        digit1,
                    ))),
                )),
            )),
        ),
        |s| f64::from_str(s)
            .expect("internal parser error")
    )(input)
}

// https://webidl.spec.whatwg.org/#prod-identifier
fn identifier(input: &str) -> IResult<&str, Identifier> {
    preceded(
        ws_and_comments,
        map(
            recognize(tuple::<&str, _, _, _>((
                opt(alt((char('_'), char('-')))),
                satisfy(|c| c.is_ascii_alphabetic()),
                many0_count(satisfy(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')),
            ))),
            |name| match name.chars().next() {
                // https://webidl.spec.whatwg.org/#idl-names
                // "For all of these constructs, the identifier is the value of the identifier token with any leading
                // U+005F LOW LINE ("_") character (underscore) removed."
                Some('_') => Identifier { name: &name[1..] },
                _ => Identifier { name },
            },
        ),
    )(input)
}

// https://webidl.spec.whatwg.org/#prod-string
fn string(input: &str) -> IResult<&str, &str> {
    preceded(ws_and_comments, delimited(char('"'), is_not("\""), char('"')))(input)
}

// https://webidl.spec.whatwg.org/#prod-whitespace
fn whitespace(input: &str) -> IResult<&str, ()> {
    value((), multispace1)(input)
}

// https://webidl.spec.whatwg.org/#prod-comment
fn comment(input: &str) -> IResult<&str, ()> {
    alt((
        value((), pair(tag("//"), not_line_ending)),
        value((), delimited(tag("/*"), take_until("*/"), tag("*/"))),
    ))(input)
}

fn ws_and_comments(input: &str) -> IResult<&str, ()> {
    fold_many0(alt((whitespace, comment)), || (), |(), ()| ())(input)
}

// https://webidl.spec.whatwg.org/#prod-other
fn other(input: &str) -> IResult<&str, char> {
    preceded(
        ws_and_comments,
        satisfy(|c|
            // NOTE: Every symbol explicitly mentioned as a token in the grammar should really
            //       be excluded here, but these are the only ones that matter for our purposes.
            c != '\t' && c != '\n' && c != '\r' && c != ' ' && !c.is_ascii_alphanumeric()
                && c != '(' && c != ')' && c != '[' && c != ']' && c != '{' && c != '}' && c != ','
        ),
    )(input)
}

/* FIXME: Remove this.
enum ExtendedAttribute<'a> {
    NoArgs(&'a str),
    ArgList(&'a str, Vec<&'a str>),
    Ident(&'a str, &'a str),
    Wildcard(&'a str),
    IdentList(&'a str, Vec<&'a str>),
    NamedArgList(&'a str, &'a str, Vec<&'a str>)
}

impl<'a> ExtendedAttribute<'a> {
    const fn lident(&self) -> &'a str {
        match *self {
            Self::NoArgs(ident) => ident,
            Self::ArgList(ident, _) => ident,
            Self::Ident(lident, _) => lident,
            Self::Wildcard(ident) => ident,
            Self::IdentList(ident, _) => ident,
            Self::NamedArgList(lident, _, _) => lident
        }
    }
}

impl<'a> From<&ExtendedAttribute<'a>> for TokenStream {
    fn from(attr: &ExtendedAttribute<'a>) -> TokenStream {
        match attr {
            ExtendedAttribute::NoArgs(ident) => {
                let ident = TokenTree::Ident(Ident::new(ident, Span::call_site()));
                quote!([$ident])
            },
            ExtendedAttribute::ArgList(ident, arg_strings) => {
                let ident = TokenTree::Ident(Ident::new(ident, Span::call_site()));
                let mut args = TokenStream::new();
                let parser = Parser::new();
                for s in arg_strings.iter() {
                    let (_, (arg, _)) = parser.argument()(s)
                        .expect("failed to parse extended attribute argument");
                    args.extend(quote!($arg,));
                }
                quote!([$ident($args)])
            },
            ExtendedAttribute::Ident(lident, rident) => {
                let lident = TokenTree::Ident(Ident::new(lident, Span::call_site()));
                let rident = TokenTree::Ident(Ident::new_raw(rident, Span::call_site()));
                quote!([$lident = $rident])
            },
            ExtendedAttribute::Wildcard(ident) => {
                let ident = TokenTree::Ident(Ident::new(ident, Span::call_site()));
                quote!([$ident = *])
            },
            ExtendedAttribute::IdentList(ident, ident_strings) => {
                let ident = TokenTree::Ident(Ident::new(ident, Span::call_site()));
                let mut idents = TokenStream::new();
                for s in ident_strings.iter() {
                    let ident = TokenTree::Ident(Ident::new_raw(s, Span::call_site()));
                    idents.extend(quote!($ident,));
                }
                quote!([$ident = ($idents)])
            },
            ExtendedAttribute::NamedArgList(lident, rident, arg_strings) => {
                let lident = TokenTree::Ident(Ident::new(lident, Span::call_site()));
                let rident = TokenTree::Ident(Ident::new_raw(rident, Span::call_site()));
                let mut args = TokenStream::new();
                let parser = Parser::new();
                for s in arg_strings.iter() {
                    let (_, (arg, _)) = parser.argument()(s)
                        .expect("failed to parse extended attribute argument");
                    args.extend(quote!($arg,));
                }
                quote!([$lident = $rident($args)])
            }
        }
    }
}*/
