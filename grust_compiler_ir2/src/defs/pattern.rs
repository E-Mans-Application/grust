//! [Pattern] module.

prelude! {}

#[derive(Debug, PartialEq, Clone)]
/// GRust matching pattern [ir1] (resemble to the [ir0]).
pub enum Pattern {
    /// Identifier pattern, gives a name to the matching expression.
    Identifier {
        /// Identifier.
        name: Ident,
    },
    /// Literal pattern, matches the given literal (constant).
    Literal {
        /// The matching literal (constant).
        literal: Constant,
    },
    /// Structure pattern that matches the structure and its fields.
    Structure {
        /// The structure path.
        path: syn::Path,
        /// The structure fields with the corresponding patterns to match.
        fields: Vec<(Ident, Pattern)>,
    },
    /// Enumeration pattern.
    Enumeration {
        /// The enumeration type name.
        enum_name: Ident,
        /// The element name.
        elem_name: Ident,
        /// The optional element of the enumeration.
        element: Option<Box<Pattern>>,
    },
    /// Tuple pattern that matches tuples.
    Tuple {
        /// The elements of the tuple.
        elements: Vec<Pattern>,
    },
    /// Ok pattern that matches when a result has a value which match the pattern.
    Ok {
        /// The pattern matching the value.
        pattern: Box<Pattern>,
    },
    /// Err pattern, matches when the result does not have a value.
    Err,
    /// Some pattern that matches when an optional has a value which match the pattern.
    Some {
        /// The pattern matching the value.
        pattern: Box<Pattern>,
    },
    /// The default pattern that matches anything.
    Default(Loc),
}

mk_new! { impl Pattern =>
    Identifier: ident { name: impl Into<Ident> = name.into() }
    Identifier: test_ident { name: impl AsRef<str> = Loc::test_id(name.as_ref()) }
    Literal: literal {literal: Constant }
    Structure: structure {
        path: impl Into<syn::Path> = path.into(),
        fields: Vec<(Ident, Self)>
    }
    Enumeration: enumeration {
        enum_name: impl Into<Ident> = enum_name.into(),
        elem_name: impl Into<Ident> = elem_name.into(),
        element: Option<Self> = element.map(Box::new),
    }
    Tuple: tuple { elements: Vec<Self> }
    Ok: ok { pattern: Self = Box::new(pattern) }
    Err: err()
    Some: some { pattern: Self = Box::new(pattern) }
    Default: default(
        loc: impl Into<Loc> = loc.into(),
    )
}

impl ToTokens for Pattern {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Pattern::Literal { literal } => match literal {
                Constant::Integer(i) => i.to_tokens(tokens),
                Constant::Float(f) => f.to_tokens(tokens),
                Constant::Boolean(b) => b.to_tokens(tokens),
                Constant::Unit(paren_token) => {
                    tokens.extend(quote_spanned! {paren_token.span => ()})
                }
                Constant::Default(loc) => {
                    tokens.extend(quote_spanned! { loc.span => Default::default() })
                }
            },
            Pattern::Identifier { name } => name.to_tokens(tokens),
            Pattern::Default(loc) => tokens.extend(quote_spanned! { loc.span => _ }),
            Pattern::Ok { pattern } => tokens.extend(quote! { Ok(#pattern) }),
            Pattern::Err => tokens.extend(quote! { Err(()) }),
            Pattern::Some { pattern } => tokens.extend(quote! { Some(#pattern) }),
            Pattern::Structure { path, fields } => {
                let fields = fields.iter().map(|(name, pattern)| quote!(#name: #pattern));
                tokens.extend(quote! {
                    #path { #(#fields),* }
                })
            }
            Pattern::Enumeration {
                enum_name,
                elem_name,
                element,
            } => {
                let element = element.as_ref().map(|e| quote!((#e)));
                tokens.extend(quote! {
                    #enum_name :: #elem_name #element
                })
            }
            Pattern::Tuple { elements } => tokens.extend(quote! {
                ( #(#elements),* )
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_create_a_rust_ast_default_pattern_from_a_ir2_default_pattern() {
        let pattern = Pattern::Default(Loc::test_dummy());
        let control = parse_quote! { _ };
        let pat: syn::Pat = parse_quote! { #pattern };
        assert_eq!(pat, control)
    }

    #[test]
    fn should_create_a_rust_ast_tuple_structure_pattern_from_a_ir2_some_pattern() {
        let pattern = Pattern::Some {
            pattern: Box::new(Pattern::Default(Loc::test_dummy())),
        };

        let control = parse_quote! { Some(_) };
        let pat: syn::Pat = parse_quote! { #pattern };
        assert_eq!(pat, control)
    }

    #[test]
    fn should_create_a_rust_ast_literal_pattern_from_a_ir2_constant_pattern() {
        let pattern = Pattern::Literal {
            literal: Constant::Integer(parse_quote!(1i64)),
        };

        let control = parse_quote! { 1i64 };
        let pat: syn::Pat = parse_quote! { #pattern };
        assert_eq!(pat, control)
    }

    #[test]
    fn should_create_a_rust_ast_ident_pattern_owned_and_immutable_from_a_ir2_ident_pattern() {
        let pattern = Pattern::test_ident("x");

        let control = parse_quote! { x };
        let pat: syn::Pat = parse_quote! { #pattern };
        assert_eq!(pat, control)
    }

    #[test]
    fn should_create_a_rust_ast_structure_pattern_from_a_ir2_structure_pattern() {
        let pattern = Pattern::Structure {
            path: Loc::test_id("Point").into(),
            fields: vec![
                (Loc::test_id("x"), Pattern::Default(Loc::test_dummy())),
                (Loc::test_id("y"), Pattern::test_ident("y")),
            ],
        };

        let control = parse_quote! { Point { x: _, y : y } };
        let pat: syn::Pat = parse_quote! { #pattern };
        assert_eq!(pat, control)
    }
}
