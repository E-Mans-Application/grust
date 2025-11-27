prelude! {
    syn::{LitInt, LitFloat, LitBool, token::Paren, Parse, Spanned},
}

/// GRust constants.
///
/// [Constant] enumeration is used to describe GRust expressions.
///
/// It represents all possible constant:
///
/// - [Constant::Integer] are [i64] integers, `1` becomes `Constant::Integer(1)`
/// - [Constant::Float] are [f64] floats, `1.0` becomes `Constant::Float(1.0)`
/// - [Constant::Boolean] is the [bool] type for booleans, `true` becomes `Constant::Boolean(true)`
/// - [Constant::Unit] is the unit type, `()` becomes `Constant::Unit`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constant {
    /// [i64] integers
    Integer(LitInt),
    /// [f64] floats
    Float(LitFloat),
    /// [bool] booleans
    Boolean(LitBool),
    /// Unit constant
    Unit(Paren),
    /// Default constant
    Default(Loc),
}

mk_new! { impl Constant =>
    Integer: int(l: LitInt = l)
    Float: float(l: LitFloat = l)
    Boolean: bool(l: LitBool = l)
    Unit: unit(l: Paren = l)
    Unit: unit_default(l = Default::default())
    Default: default(loc: impl Into<Loc> = loc.into())
}

impl ToTokens for Constant {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Constant::Integer(i) => {
                let i = LitInt::new(&(i.base10_digits().to_owned() + "i64"), i.span());
                tokens.extend(quote!(#i))
            }
            Constant::Float(f) => {
                let f = if f.suffix() == "" {
                    let mut s = f.to_string();
                    // careful on trailing `.`
                    if s.ends_with(".") {
                        s.push('0');
                    }
                    s.push_str("f64");
                    syn::LitFloat::new(&s, f.span())
                } else {
                    f.clone()
                };

                tokens.extend(quote!(#f))
            }
            Constant::Boolean(b) => b.to_tokens(tokens),
            Constant::Unit(paren_token) => tokens.extend(quote_spanned!(paren_token.span => ())),
            Constant::Default(loc) => tokens.extend(quote_spanned!(loc.span => Default::default())),
        }
    }
}
impl ToLogicTokens for Constant {
    fn to_logic_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Constant::Integer(i) => i.to_tokens(tokens),
            Constant::Float(f) => f.to_tokens(tokens),
            Constant::Boolean(b) => b.to_tokens(tokens),
            Constant::Unit(paren_token) => tokens.extend(quote_spanned!(paren_token.span => ())),
            Constant::Default(loc) => tokens.extend(quote_spanned!(loc.span => Default::default())),
        }
    }
}

impl Constant {
    pub fn loc(&self) -> Loc {
        match self {
            Self::Integer(l) => l.span().into(),
            Self::Float(l) => l.span().into(),
            Self::Boolean(l) => l.span().into(),
            Self::Unit(p) => p.span.join().into(),
            Self::Default(loc) => *loc,
        }
    }

    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default(_))
    }

    /// The `syn` version of a constant.
    pub fn into_syn(self) -> syn::Expr {
        prelude!(syn::{Expr, ExprLit, ExprTuple});
        match self {
            Constant::Integer(i) => Expr::Lit(ExprLit {
                attrs: vec![],
                lit: syn::Lit::Int(LitInt::new(
                    &(i.base10_digits().to_owned() + "i64"),
                    i.span(),
                )),
            }),
            Constant::Float(f) => Expr::Lit(ExprLit {
                attrs: vec![],
                lit: {
                    // force `f64` suffix
                    let f = if f.suffix() == "" {
                        let mut s = f.to_string();
                        // careful on trailing `.`
                        if s.ends_with(".") {
                            s.push('0');
                        }
                        s.push_str("f64");
                        syn::LitFloat::new(&s, f.span())
                    } else {
                        f.clone()
                    };
                    syn::Lit::Float(f)
                },
            }),
            Constant::Boolean(b) => Expr::Lit(ExprLit {
                attrs: vec![],
                lit: syn::Lit::Bool(b.clone()),
            }),
            Constant::Unit(paren_token) => Expr::Tuple(ExprTuple {
                attrs: vec![],
                paren_token,
                elems: Default::default(),
            }),
            Constant::Default(loc) => parse_quote_spanned! {loc.span => Default::default() },
        }
    }

    /// The `logic` version of a constant.
    pub fn into_logic(self) -> syn::Expr {
        prelude!(syn::{Expr, ExprLit, ExprTuple});
        match self {
            Constant::Integer(i) => Expr::Lit(ExprLit {
                attrs: vec![],
                lit: syn::Lit::Int(LitInt::new(i.base10_digits(), i.span())),
            }),
            Constant::Float(f) => Expr::Lit(ExprLit {
                attrs: vec![],
                lit: {
                    // force `f64` suffix
                    let f = if f.suffix() == "" {
                        let mut s = f.to_string();
                        // careful on trailing `.`
                        if s.ends_with(".") {
                            s.push('0');
                        }
                        syn::LitFloat::new(&s, f.span())
                    } else {
                        f.clone()
                    };
                    syn::Lit::Float(f)
                },
            }),
            Constant::Boolean(b) => Expr::Lit(ExprLit {
                attrs: vec![],
                lit: syn::Lit::Bool(b.clone()),
            }),
            Constant::Unit(paren_token) => Expr::Tuple(ExprTuple {
                attrs: vec![],
                paren_token,
                elems: Default::default(),
            }),
            Constant::Default(loc) => parse_quote_spanned! {loc.span => Default::default() },
        }
    }

    /// Get the [Typ] of the constant.
    pub fn get_typ(&self) -> Typ {
        match self {
            Constant::Integer(lit) => Typ::Integer(keyword::int { span: lit.span() }),
            Constant::Float(lit) => Typ::Float(keyword::float { span: lit.span() }),
            Constant::Boolean(lit) => Typ::Boolean(keyword::bool { span: lit.span() }),
            Constant::Unit(paren) => Typ::Unit(keyword::unit {
                span: paren.span.span(),
            }),
            Constant::Default(_) => Typ::Any,
        }
    }

    /// Get the [ReactTyp] of the constant.
    pub fn get_react_typ(&self) -> ReactTyp {
        ReactTyp::d()
    }

    pub fn peek(input: ParseStream) -> bool {
        input.peek(LitInt) || input.peek(LitFloat) || input.peek(LitBool) || {
            (|| {
                let content;
                let _ = parenthesized!(content in input.fork());
                Ok(content)
            })()
            .is_ok_and(|content| content.is_empty())
        }
    }
}
impl Parse for Constant {
    fn parse(input: ParseStream) -> syn::Res<Self> {
        if input.peek(LitInt) {
            let i: LitInt = input.parse()?;
            Ok(Constant::Integer(i))
        } else if input.peek(LitFloat) {
            let f: LitFloat = input.parse()?;
            Ok(Constant::Float(f))
        } else if input.peek(LitBool) {
            let b: LitBool = input.parse()?;
            Ok(Constant::Boolean(b))
        } else {
            let content;
            let parens = parenthesized!(content in input);
            if content.is_empty() {
                Ok(Constant::Unit(parens))
            } else {
                Err(input.error("expected unit `()`"))
            }
        }
    }
}
