//! Basic types, helpers and re-exports.

/// Imports the compiler prelude.
#[macro_export]
macro_rules! prelude {
    { just $($imports:tt)* } => {
        use $crate::prelude::{$($imports)*};
    };
    { $($imports:tt)* } => {
        use $crate::prelude::{*, $($imports)*};
    };
}

#[macro_export]
macro_rules! todoo {
    {  } => {
        unimplemented!("at `{}:{}`", file!(), line!())
    };
    { $($blah:tt)* } => {
        unimplemented!("at `{}:{}`, {}", file!(), line!(), format!($($blah)*))
    };
}

#[macro_export]
macro_rules! noteln {
    { $($stuff:tt)* } => {
        error!($($stuff)*).emit_note()
    }
}

#[macro_export]
macro_rules! res_vec {
    { $len:expr, $e:expr $(,)? } => {{
        let mut vec = Vec::with_capacity($len);
        for res in $e {
            vec.push(res?);
        }
        vec
    }};
    { $e:expr $(,)? } => {{
        for res in $e {
            res?;
        }
    }};
}

/// Same as [syn::Token] but generates a value with a default span.
#[macro_export]
macro_rules! token {
    { $($stuff:tt)* } => {
        $crate::syn::Token![$($stuff)*](Span::call_site())
    };
}

/// Iterates over a collection with a special action if there is only one element.
///
/// # Examples
///
/// ```rust
/// # use grust_compiler_common::iter_1;
/// fn run(vec: Vec<usize>) -> usize {
///     iter_1!(
///         // `Iterator + ExactSizeIterator` to work on
///         vec.into_iter(),
///         // action when the length isn't one
///         |coll| coll.sum(),
///         // action when the iterator contains a single element
///         |elem| elem * 10
///     )
/// }
/// let coll_1 = vec![2];
/// assert_eq!( run(coll_1), 20 );
/// let coll_n = vec![2, 3, 4];
/// assert_eq!( run(coll_n), 9 );
/// ```
#[macro_export]
macro_rules! iter_1 {
    { $e:expr, |$iter:ident| $iter_do:expr, |$elem:ident| $one_do:expr $(,)? } => {{
        let mut iter = $e;
        if iter.len() == 1 {
            let $elem = iter.next().expect("internal error: len is `1`");
            $one_do
        } else {
            let $iter = iter;
            $iter_do
        }
    }}
}

pub use std::{
    collections::{BTreeMap, BTreeSet},
    error,
    fmt::Display,
    hash::Hash,
    ops,
    time::{Duration, Instant},
};

pub mod syn {
    pub use syn::{
        braced, bracketed, custom_keyword, parenthesized,
        parse::{Parse, ParseStream},
        parse_macro_input, parse_quote, parse_quote_spanned, parse_str,
        punctuated::{self, Punctuated},
        spanned::Spanned,
        Error, Token, *,
    };

    /// Alias for `syn`'s notion of result.
    pub type Res<T> = syn::Result<T>;
}

pub use macro1;
pub use macro2::{Span, TokenStream as TokenStream2};
pub use quote::{format_ident, quote, quote_spanned, ToTokens};
pub use serde::{Deserialize, Serialize};
pub use syn::{
    braced, bracketed, custom_keyword, parenthesized, parse_macro_input, parse_quote,
    parse_quote_spanned, Ident, ParseStream, Token,
};

pub use crate::{
    bad,
    // error::*,
    bail,
    check,
    // codespan_reporting,
    conf::{self, Conf},
    constant::Constant,
    convert_case::{to_camel_case, to_snake_case},
    either::Either,
    err::*,
    error,
    ext::*,
    graph,
    hash_map::*,
    iter_1,
    itertools,
    json,
    keyword,
    lazy_static::lazy_static,
    lerror,
    lnote,
    macro2,
    mk_new,
    noErrorDesc,
    note,
    noteln,
    once_cell,
    op::{BOp, OtherOp, UOp},
    petgraph,
    quote,
    react_typ::ReactTyp,
    res_vec,
    rustc_hash,
    scope::Scope,
    serde,
    stats::*,
    strum,
    synced,
    todoo,
    token,
    typ::Typ,
    w8,
};

/// Custom location type, wraps a [`Span`].
#[derive(Debug, Clone, Copy)]
pub struct Loc {
    /// Inner location as a span.
    pub span: Span,
}
impl Loc {
    pub fn test_id(id: impl AsRef<str>) -> Ident {
        Ident::new(id.as_ref(), Self::test_dummy().into())
    }
    pub fn test_dummy() -> Self {
        Self {
            span: Span::mixed_site(),
        }
    }
    pub fn nu_call_site() -> Self {
        Self {
            span: Span::call_site(),
        }
    }
    pub fn builtin() -> Self {
        Self {
            span: Span::call_site(),
        }
    }
    pub fn builtin_id(id: impl AsRef<str>) -> Ident {
        Ident::new(id.as_ref(), Self::builtin().into())
    }
    pub fn try_join(self, that: impl Into<Self>) -> Option<Self> {
        self.span.join(that.into().span).map(Loc::from)
    }
    pub fn join(self, that: impl Into<Self>) -> Self {
        self.try_join(that).unwrap_or(self)
    }
}
impl PartialEq for Loc {
    fn eq(&self, other: &Self) -> bool {
        // TODO: that's pretty bad, but we can't have `PartialEq` on `Span` itself...
        format!("{:?}", self.span) == format!("{:?}", other.span)
    }
}
impl Eq for Loc {}
impl std::hash::Hash for Loc {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // TODO: that's pretty bad, but we can't have `Hash` on `Span` itself...
        format!("{:?}", self.span).hash(state)
    }
}
impl std::ops::Deref for Loc {
    type Target = Span;
    fn deref(&self) -> &Self::Target {
        &self.span
    }
}
impl From<Span> for Loc {
    fn from(span: Span) -> Self {
        Self { span }
    }
}
impl From<Loc> for Span {
    fn from(span: Loc) -> Self {
        span.span
    }
}
pub fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// Provides context-dependent a *less than* (`lt`) relation.
pub trait Lt {
    /// Type of the context.
    type Ctx;

    /// Compares two `Self`-values.
    fn lt(&self, other: &Self, ctx: &Self::Ctx) -> bool;
}

pub trait MaybeHasLoc {
    fn loc_opt(&self) -> Option<Loc>;
}

/// "Logic" version of a part of the AST.
///
/// This is only used to implement `ToTokens<Logic<T>>` for the "logic" version of the tokens of
/// `T`.
pub struct Logic<'a, T> {
    /// `T` value.
    pub val: &'a T,
}

/// Tokens for the "logic" version of a part of the AST.
pub trait ToLogicTokens: Sized {
    /// Turns a `T` value into its "logic" version.
    fn to_logic(&self) -> Logic<Self> {
        Logic { val: self }
    }
    /// Turns a `T` value into its "logic" version as tokens.
    fn to_logic_tokens(&self, tokens: &mut TokenStream2);
}
impl<T: ToLogicTokens> ToTokens for Logic<'_, T> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.val.to_logic_tokens(tokens);
    }
}

/// "Prefix" version of a part of the AST.
///
/// This is only used to implement `ToTokens<Prefix<T>>` for the "prefixed" version of the tokens of
/// `T`.
pub struct Prefix<'a, T> {
    pub path: &'a syn::Path,
    /// `T` value.
    pub val: &'a T,
}

/// Tokens for the "prefix" version of a part of the AST.
pub trait ToPrefixTokens: Sized {
    /// Turns a `T` value into its "prefix" version.
    fn to_prefix<'a>(&'a self, path: &'a syn::Path) -> Prefix<'a, Self> {
        Prefix { path, val: self }
    }
    /// Turns a `T` value into its "prefix" version as tokens.
    fn to_prefix_tokens(&self, path: &syn::Path, tokens: &mut TokenStream2);
}
impl<T: ToPrefixTokens> ToTokens for Prefix<'_, T> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.val.to_prefix_tokens(self.path, tokens);
    }
}

/// Byte-level levenshtein distance, not sure this makes sense for unicode.
///
/// Stackless version of <https://en.wikipedia.org/wiki/Levenshtein_distance#Recursive>.
pub fn levenshtein(s1: impl AsRef<str>, s2: impl AsRef<str>) -> usize {
    struct TwoBytes<'a>(&'a [u8], &'a [u8]);
    enum Frame<'a> {
        TwoLeft {
            next: TwoBytes<'a>,
            last: TwoBytes<'a>,
        },
        OneLeft {
            min: usize,
            next: TwoBytes<'a>,
        },
        None {
            min: usize,
        },
    }
    let (mut s1, mut s2) = (s1.as_ref().as_bytes(), s2.as_ref().as_bytes());
    let mut stack: Vec<Frame> = Vec::with_capacity(s1.len().max(s2.len()));
    'measure: loop {
        // println!("measure");
        // println!("- `{}`", String::from_utf8_lossy(s1));
        // println!("- `{}`", String::from_utf8_lossy(s2));
        let mut distance = if s1.is_empty() {
            s2.len()
        } else if s2.is_empty() {
            s1.len()
        } else if s1[0] == s2[0] {
            s1 = &s1[1..];
            s2 = &s2[1..];
            continue 'measure;
        } else {
            stack.push(Frame::TwoLeft {
                next: TwoBytes(&s1[1..], s2),
                last: TwoBytes(&s1[1..], &s2[1..]),
            });
            s2 = &s2[1..];
            continue 'measure;
        };

        'unstack: loop {
            let TwoBytes(next1, next2) = match stack.pop() {
                None => return distance,
                Some(Frame::TwoLeft { next, last }) => {
                    stack.push(Frame::OneLeft {
                        min: distance,
                        next: last,
                    });
                    next
                }
                Some(Frame::OneLeft { min, next }) => {
                    stack.push(Frame::None {
                        min: distance.min(min),
                    });
                    next
                }
                Some(Frame::None { min }) => {
                    distance = 1 + distance.min(min);
                    continue 'unstack;
                }
            };
            s1 = next1;
            s2 = next2;
            continue 'measure;
        }
    }
}

#[test]
fn test_levenshtein() {
    fn check(s1: &str, s2: &str, exp: usize) {
        let l = levenshtein(s1, s2);
        println!();
        println!("- {} =?= {}", l, exp);
        println!("  `{}`", s1);
        println!("  `{}`", s2);
        assert_eq!(l, exp);
        println!("  ✅")
    }
    check("a", "a", 0);
    check("a", "b", 1);
    check("i12", "i76", 2);
    check("kitten", "sitting", 3);
    check("uninformed", "uniformed", 1);
}
