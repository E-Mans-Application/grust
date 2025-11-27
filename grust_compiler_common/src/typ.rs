prelude! {
    syn::{Parse, Punctuated, token},
}

/// GRust types.
///
/// [Typ] enumeration used when typing a GRust program.
///
/// It represents all possible types a GRust expression can take:
///
/// - [Typ::Integer] are [i64] integers, if `n = 1` then `n: int`
/// - [Typ::Float] are [f64] floats, if `r = 1.0` then `r: float`
/// - [Typ::Boolean] is the [bool] type for booleans, if `b = true` then `b: bool`
/// - [Typ::Unit] is the unit type, if `u = ()` then `u: unit`
/// - [Typ::Array] is the array type, if `a = [1, 2, 3]` then `a: [int; 3]`
/// - [Typ::Option] is the event type for StateMachine, noted `n: int?`
/// - [Typ::Enumeration] is a user-defined enumeration:
///   - if `c = Color.Yellow`,
///   - then `c: Enumeration(Color)`.
/// - [Typ::Structure] is a user-defined structure:
///   - if `p = Point { x: 1, y: 0}`,
///   - then `p: Structure(Point)`.
/// - [Typ::NotDefinedYet] is not defined yet, if `x: Color` then `x: NotDefinedYet(Color)`
/// - [Typ::Fn] are functions types, if `f = |x| x+1` then `f: int -> int`
/// - [Typ::Polymorphism] is an inferable function type:
///   - if `add = |x, y| x+y`,
///   - then `add: 't -> 't -> 't` with `'t` in `{int, float}`.
#[allow(clippy::derived_hash_with_manual_eq)]
#[derive(Debug, Eq, Hash, Clone)]
pub enum Typ {
    /// [i64] integers, if `n = 1` then `n: int`
    Integer(keyword::int),
    /// [f64] floats, if `r = 1.0` then `r: float`
    Float(keyword::float),
    /// [bool] type for booleans, if `b = true` then `b: bool`
    Boolean(keyword::bool),
    /// Unit type, if `u = ()` then `u: unit`
    Unit(keyword::unit),
    /// Array type, if `a = [1, 2, 3]` then `a: [int; 3]`
    Array {
        bracket_token: token::Bracket,
        ty: Box<Typ>,
        semi_token: Token![;],
        size: syn::LitInt,
    },
    /// Option type, noted `n: int?`
    Option {
        ty: Box<Typ>,
        question_token: Token![?],
    },
    /// User defined enumeration, if `c = Color.Yellow` then `c: Enumeration(Color)`
    Enumeration {
        /// Enumeration's name.
        name: Ident,
        /// Enumeration's identifier.
        id: usize,
    },
    /// User defined structure, if `p = Point { x: 1, y: 0}` then `p: Structure(Point)`
    Structure {
        /// Structure's name.
        name: Ident,
        /// Structure's identifier.
        id: usize,
    },
    /// Functions types, if `f = |x| x+1` then `f: int -> int`
    Fn {
        paren_token: Option<token::Paren>,
        inputs: Punctuated<Typ, Token![,]>,
        arrow_token: Token![->],
        output: Box<Typ>,
    },
    /// Tuple type, if `z = zip(a, b)` with `a: [int; 5]` and `b: [float; 5]` then
    /// `z: [(int, float); 5]`
    Tuple {
        paren_token: token::Paren,
        elements: Punctuated<Typ, Token![,]>,
    },
    /// Signal type, in interface if `s' = map(s, |x| x + 1)` then `s': signal int`
    Signal {
        signal_token: keyword::signal,
        ty: Box<Typ>,
    },
    /// Event type, in interface if `e' = map(e, |x| x + 1)` then `e': event int`
    Event {
        event_token: keyword::event,
        ty: Box<Typ>,
    },
    /// Not defined yet, if `x: Color` then `x: NotDefinedYet(Color)`
    NotDefinedYet(Ident),
    /// Polymorphic type, if `add = |x, y| x+y` then `add: 't : Typ -> t -> 't -> 't`
    Polymorphism(fn(Vec<Typ>, Loc) -> Res<Typ>),
    /// Match any type.
    Any,
}
impl PartialEq for Typ {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Any, _) => true,
            (_, Self::Any) => true,
            (Self::Integer(_), Self::Integer(_))
            | (Self::Float(_), Self::Float(_))
            | (Self::Boolean(_), Self::Boolean(_))
            | (Self::Unit(_), Self::Unit(_))
            | (Self::NotDefinedYet(_), Self::NotDefinedYet(_))
            | (Self::Polymorphism(_), Self::Polymorphism(_)) => true,
            (
                Self::Array {
                    ty: l_ty,
                    size: l_size,
                    ..
                },
                Self::Array {
                    ty: r_ty,
                    size: r_size,
                    ..
                },
            ) => l_ty == r_ty && l_size == r_size,
            (Self::Option { ty: l_ty, .. }, Self::Option { ty: r_ty, .. }) => l_ty == r_ty,
            (
                Self::Enumeration {
                    name: l_name,
                    id: l_id,
                },
                Self::Enumeration {
                    name: r_name,
                    id: r_id,
                },
            ) => *l_name == *r_name && l_id == r_id,
            (
                Self::Structure {
                    name: l_name,
                    id: l_id,
                },
                Self::Structure {
                    name: r_name,
                    id: r_id,
                },
            ) => *l_name == *r_name && l_id == r_id,
            (
                Self::Fn {
                    inputs: l_inputs,
                    output: l_output,
                    ..
                },
                Self::Fn {
                    inputs: r_inputs,
                    output: r_output,
                    ..
                },
            ) => l_inputs.iter().zip(r_inputs).all(|(a, b)| a == b) && l_output == r_output,
            (
                Self::Tuple {
                    elements: l_elements,
                    ..
                },
                Self::Tuple {
                    elements: r_elements,
                    ..
                },
            ) => l_elements.iter().zip(r_elements).all(|(a, b)| a == b),
            (Self::Signal { ty: l_ty, .. }, Self::Signal { ty: r_ty, .. }) => l_ty == r_ty,
            (Self::Event { ty: l_ty, .. }, Self::Event { ty: r_ty, .. }) => l_ty == r_ty,
            _ => false,
        }
    }
}
impl Display for Typ {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Typ::Integer(_) => write!(f, "i64"),
            Typ::Float(_) => write!(f, "f64"),
            Typ::Boolean(_) => write!(f, "bool"),
            Typ::Unit(_) => write!(f, "unit"),
            Typ::Array { ty, size, .. } => write!(f, "[{}; {size}]", *ty),
            Typ::Option { ty, .. } => write!(f, "Option<{}>", *ty),
            Typ::Enumeration { name, .. } => write!(f, "{name}"),
            Typ::Structure { name, .. } => write!(f, "{name}"),
            Typ::Fn { inputs, output, .. } => {
                write!(f, "(")?;
                let mut sep = "";
                for typ in inputs {
                    write!(f, "{sep}{typ}")?;
                    sep = ", "
                }
                write!(f, ") -> {}", *output)
            }
            Typ::Tuple { elements, .. } => {
                write!(f, "(")?;
                let mut sep = "";
                for typ in elements {
                    write!(f, "{sep}{typ}")?;
                    sep = ", "
                }
                write!(f, ")")
            }
            Typ::Signal { ty, .. } => write!(f, "Signal<{}>", *ty),
            Typ::Event { ty, .. } => write!(f, "Event<{}>", *ty),
            Typ::NotDefinedYet(s) => write!(f, "{s}"),
            Typ::Polymorphism(v_t) => write!(f, "{v_t:#?}"),
            Typ::Any => write!(f, "any"),
        }
    }
}
impl Parse for Typ {
    fn parse(input: ParseStream) -> syn::Res<Self> {
        let mut ty = if input.peek(keyword::int) {
            let keyword: keyword::int = input.parse()?;
            Typ::Integer(keyword)
        } else if input.peek(keyword::float) {
            let keyword: keyword::float = input.parse()?;
            Typ::Float(keyword)
        } else if input.peek(keyword::bool) {
            let keyword: keyword::bool = input.parse()?;
            Typ::Boolean(keyword)
        } else if input.peek(keyword::unit) {
            let keyword: keyword::unit = input.parse()?;
            Typ::Unit(keyword)
        } else if input.peek(token::Paren) {
            let content;
            let paren_token = parenthesized!(content in input);
            let elements: Punctuated<Typ, Token![,]> = Punctuated::parse_terminated(&content)?;
            Typ::Tuple {
                paren_token,
                elements,
            }
        } else if input.peek(token::Bracket) {
            let content;
            let bracket_token = bracketed!(content in input);
            if content.is_empty() {
                return Err(input.error("expected type: `int`, `float`, etc"));
            } else {
                let ty = content.parse()?;
                let semi_token: Token![;] = content.parse()?;
                let size: syn::LitInt = content.parse()?;
                Typ::Array {
                    bracket_token,
                    ty: Box::new(ty),
                    semi_token,
                    size,
                }
            }
        } else {
            let ident: Ident = input.parse()?;
            Typ::NotDefinedYet(ident)
        };

        loop {
            if input.peek(Token![?]) {
                let question_token: Token![?] = input.parse()?;
                ty = Typ::Option {
                    ty: Box::new(ty),
                    question_token,
                }
            } else if input.peek(Token![->]) {
                let arrow_token: Token![->] = input.parse()?;
                let out_ty = input.parse()?;
                ty = match ty {
                    Typ::Tuple {
                        paren_token,
                        elements,
                    } => Typ::Fn {
                        paren_token: Some(paren_token),
                        inputs: elements,
                        arrow_token,
                        output: Box::new(out_ty),
                    },
                    _ => {
                        let mut inputs = syn::Punctuated::new();
                        inputs.push_value(ty);
                        Typ::Fn {
                            paren_token: None,
                            inputs,
                            arrow_token,
                            output: Box::new(out_ty),
                        }
                    }
                }
            } else {
                break;
            }
        }

        Ok(ty)
    }
}

mk_new! { impl Typ =>
    Integer: int (
        keyword = Default::default()
    )
    Float: float (
        keyword = Default::default()
    )
    Boolean: bool (
        keyword = Default::default()
    )
    Unit: unit (
        keyword = Default::default()
    )
    Array: array {
        bracket_token = Default::default(),
        ty: Typ = ty.into(),
        semi_token = Default::default(),
        size: usize = syn::LitInt::new(&format!("{size}"), Span::call_site()),
    }
    Enumeration: enumeration {
        name: Ident,
        id: usize,
    }
    Structure: structure {
        name: Ident,
        id: usize,
    }
    Enumeration: enumeration_str {
        name: impl Into<Ident> = name.into(),
        id: usize,
    }
    Structure: structure_str {
        name: impl Into<Ident> = name.into(),
        id: usize,
    }
    Fn: function {
        paren_token = Default::default(),
        inputs: Vec<Typ> = {
            let mut args = Punctuated::new();
            args.extend(inputs);
            args
        },
        arrow_token = Default::default(),
        output: Typ = output.into(),
    }
    Tuple: tuple {
        paren_token = Default::default(),
        elements: Vec<Typ> = {
            let mut tys = Punctuated::new();
            tys.extend(elements);
            tys
        },
    }
    Option: option {
        ty: Typ = ty.into(),
        question_token = Default::default(),
    }
    Signal: signal {
        signal_token = Default::default(),
        ty: Typ = ty.into(),
    }
    Event: event {
        event_token = Default::default(),
        ty: Typ = ty.into(),
    }
    NotDefinedYet: undef(
        name: impl Into<String> = Ident::new(&name.into(), Span::call_site())
    )
    Polymorphism: poly(f : fn(Vec<Self>, Loc) -> Res<Self> = f)
    Any: any()
}

pub mod typ_tokens {
    use super::*;

    impl ToTokens for Typ {
        fn to_tokens(&self, tokens: &mut TokenStream2) {
            match self {
                Typ::Integer(_) => quote!(i64).to_tokens(tokens),
                Typ::Float(_) => quote!(f64).to_tokens(tokens),
                Typ::Boolean(_) => quote!(bool).to_tokens(tokens),
                Typ::Unit(_) => quote!(()).to_tokens(tokens),
                Typ::Enumeration { name, .. } | Typ::Structure { name, .. } => {
                    name.to_tokens(tokens)
                }
                Typ::Array { ty, size, .. } => {
                    let size = syn::Lit::Int(syn::LitInt::new(
                        &(size.base10_digits().to_owned() + "usize"),
                        size.span(),
                    ));
                    quote!( [#ty; #size] ).to_tokens(tokens)
                }
                Typ::Fn { inputs, output, .. } => {
                    let inputs = inputs.iter();
                    quote!(impl Fn(#(#inputs),*) -> #output).to_tokens(tokens)
                }
                Typ::Tuple { elements, .. } => {
                    let elements = elements.iter();
                    quote!((#(#elements),*)).to_tokens(tokens)
                }
                Typ::Event { ty, .. } | Typ::Signal { ty, .. } => ty.to_tokens(tokens),
                Typ::Option { ty, .. } => quote!(Option<#ty>).to_tokens(tokens),
                Typ::NotDefinedYet(_) | Typ::Polymorphism(_) | Typ::Any => {
                    noErrorDesc!()
                }
            }
        }
    }

    impl ToLogicTokens for Typ {
        fn to_logic_tokens(&self, tokens: &mut TokenStream2) {
            match self {
                Typ::Integer(_) => quote!(Int).to_tokens(tokens),
                Typ::Float(_) => quote!(Float).to_tokens(tokens),
                Typ::Array { ty, size, .. } => {
                    let ty = ty.to_logic();
                    quote!([#ty; #size]).to_tokens(tokens)
                }
                Typ::Tuple { elements, .. } => {
                    let elements = elements.iter().map(|ty| ty.to_logic());
                    quote!((#(#elements),*)).to_tokens(tokens)
                }
                Typ::Option { ty, .. } => {
                    let ty = ty.to_logic();
                    quote!(Option<#ty>).to_tokens(tokens)
                }
                Typ::Boolean(_)
                | Typ::Unit(_)
                | Typ::Enumeration { .. }
                | Typ::Structure { .. } => self.to_tokens(tokens),
                Typ::Fn { .. }
                | Typ::Event { .. }
                | Typ::Signal { .. }
                | Typ::NotDefinedYet(_)
                | Typ::Polymorphism(_)
                | Typ::Any => {
                    noErrorDesc!()
                }
            }
        }
    }

    impl ToPrefixTokens for Typ {
        fn to_prefix_tokens(&self, path: &syn::Path, tokens: &mut TokenStream2) {
            match self {
                Typ::Integer(_) => quote!(i64).to_tokens(tokens),
                Typ::Float(_) => quote!(f64).to_tokens(tokens),
                Typ::Boolean(_) => quote!(bool).to_tokens(tokens),
                Typ::Unit(_) => quote!(()).to_tokens(tokens),
                Typ::Enumeration { name, .. } | Typ::Structure { name, .. } => {
                    quote!(#path::#name).to_tokens(tokens)
                }
                Typ::Array { ty, size, .. } => {
                    let size = syn::Lit::Int(syn::LitInt::new(
                        &(size.base10_digits().to_owned() + "usize"),
                        size.span(),
                    ));
                    let ty = ty.to_prefix(path);
                    quote!( [#ty; #size] ).to_tokens(tokens)
                }
                Typ::Fn { inputs, output, .. } => {
                    let inputs = inputs.iter().map(|ty| ty.to_prefix(path));
                    let output = output.to_prefix(path);
                    quote!(impl Fn(#(#inputs),*) -> #output).to_tokens(tokens)
                }
                Typ::Tuple { elements, .. } => {
                    let elements = elements.iter().map(|ty| ty.to_prefix(path));
                    quote!((#(#elements),*)).to_tokens(tokens)
                }
                Typ::Event { ty, .. } | Typ::Signal { ty, .. } => ty.to_prefix_tokens(path, tokens),
                Typ::Option { ty, .. } => {
                    let ty = ty.to_prefix(path);
                    quote!(Option<#ty>).to_tokens(tokens)
                }
                Typ::NotDefinedYet(_) | Typ::Polymorphism(_) | Typ::Any => {
                    noErrorDesc!()
                }
            }
        }
    }
}

impl Typ {
    /// Tells if a conversion into logical model of Creusot is needed.
    pub fn needs_view(&self) -> bool {
        match self {
            Typ::Integer(_) | Typ::Float(_) | Typ::Array { .. } | Typ::Tuple { .. } => true,
            Typ::Boolean(_)
            | Typ::Unit(_)
            | Typ::Enumeration { .. }
            | Typ::Structure { .. }
            | Typ::Option { .. } => false,
            Typ::Fn { .. }
            | Typ::Event { .. }
            | Typ::Signal { .. }
            | Typ::NotDefinedYet(_)
            | Typ::Polymorphism(_)
            | Typ::Any => {
                noErrorDesc!()
            }
        }
    }

    /// The location of this type, if any.
    pub fn loc(&self) -> Option<Loc> {
        match self {
            Typ::Integer(kw) => Some(kw.span.into()),
            Typ::Float(kw) => Some(kw.span.into()),
            Typ::Boolean(kw) => Some(kw.span.into()),
            Typ::Unit(kw) => Some(kw.span.into()),
            Typ::Array { bracket_token, .. } => Some(bracket_token.span.join().into()),
            Typ::Option { ty, question_token } => ty.loc()?.try_join(question_token.span),
            Typ::Enumeration { name, .. } | Typ::Structure { name, .. } => Some(name.span().into()),
            Typ::Fn {
                paren_token: Some(paren),
                ..
            } => Some(paren.span.join().into()),
            Typ::Fn { inputs, output, .. } => inputs.first()?.loc()?.try_join(output.loc()?),
            Typ::Tuple { paren_token, .. } => Some(paren_token.span.join().into()),
            Typ::Event { ty, event_token } => ty.loc()?.try_join(event_token.span),
            Typ::Signal { ty, signal_token } => ty.loc()?.try_join(signal_token.span),
            Typ::NotDefinedYet(id) => Some(id.span().into()),
            Typ::Polymorphism(_) | Typ::Any => None,
        }
    }

    /// The location of this type if any, `loc` otherwise.
    pub fn loc_or(&self, loc: Loc) -> Loc {
        self.loc().unwrap_or(loc)
    }

    /// True if this type is arithmetic-like.
    pub fn is_arith_like(&self) -> bool {
        match self {
            Self::Integer(_) | Self::Float(_) | Self::Any => true,
            Typ::Boolean(_)
            | Typ::Unit(_)
            | Typ::Array { .. }
            | Typ::Option { .. }
            | Typ::Enumeration { .. }
            | Typ::Structure { .. }
            | Typ::Fn { .. }
            | Typ::Tuple { .. }
            | Typ::Event { .. }
            | Typ::Signal { .. }
            | Typ::NotDefinedYet(_)
            | Typ::Polymorphism(_) => false,
        }
    }

    /// Fails if `!self.is_arith_like()` at location `loc`.
    pub fn check_arith_like(&self, loc: Loc) -> URes {
        check::typ::arith_like(loc, self)
    }

    pub fn expect(&self, loc: Loc, expected: &Self) -> URes {
        check::typ::expect(loc, self, expected)
    }

    pub fn expect_bool(&self, loc: Loc) -> URes {
        check::typ::expect(loc, self, &Self::bool())
    }

    /// Type application with errors handling.
    ///
    /// This function tries to apply the input type to the self type. If types are incompatible for
    /// application then an error is raised.
    ///
    /// In case of a [Typ::Polymorphism], it redefines the type according to the inputs.
    ///
    /// # Example
    /// ```rust
    /// # grust_compiler_common::prelude! {}
    /// let mut errors = vec![];
    ///
    /// let input_types = vec![Typ::int()];
    /// let output_type = Typ::bool();
    /// let mut fn_type = Typ::function(input_types.clone(), output_type.clone());
    ///
    /// let application_result = fn_type
    ///     .apply(input_types, Loc::test_dummy(), &mut errors)
    ///     .unwrap();
    ///
    /// assert_eq!(application_result, output_type);
    /// ```
    pub fn apply(&mut self, input_types: Vec<Typ>, loc: Loc, errors: &mut Vec<Error>) -> TRes<Typ> {
        match self {
            // if self is a lambda, check if the input types are equal
            // and return the output type as the type of the application
            Typ::Fn { inputs, output, .. } => {
                check::arity::expect(loc, input_types.len(), inputs.len()).move_err(errors);
                let mut fail = false;
                for (idx, (typ, expected)) in input_types.iter().zip(inputs).enumerate() {
                    let res = typ
                        .expect(loc, expected)
                        .err_note(lnote!(
                            @loc => "on argument #{} of this lambda application", idx + 1,
                        ))
                        .dewrap(errors);
                    if res.is_err() {
                        fail = true;
                    }
                }
                if fail {
                    bad!()
                } else {
                    Ok((**output).clone())
                }
            }
            // if self is a polymorphic type, apply the function returning the function_type with
            // the input_types, then apply the function_type with the input_type just like any other
            // type
            Typ::Polymorphism(fn_typ) => {
                let mut out_typ = fn_typ(input_types.clone(), loc).dewrap(errors)?;
                let result = out_typ.apply(input_types.clone(), loc, errors)?;
                *self = out_typ;
                Ok(result)
            }
            _ => {
                errors.push(error!(
                    @loc => ErrorKind::expected_lambda(input_types, self.clone())
                ));
                bad!()
            }
        }
    }

    /// Check if `self` matches the expected [Typ].
    ///
    /// See also the [`check::typ`] module.
    ///
    /// # Example
    ///
    /// ```rust
    /// # grust_compiler_common::prelude! {}
    /// let given_type = Typ::int();
    /// let expected_type = Typ::int();
    ///
    /// given_type.check_eq(&expected_type, Loc::test_dummy()).unwrap();
    /// ```
    pub fn check_eq(&self, expected: &Typ, loc: Loc) -> URes {
        check::typ::expect(loc, self, expected)
    }

    /// Get inputs from lambda type.
    ///
    /// - returns a copy of lambda type inputs;
    /// - panics if not lambda type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # grust_compiler_common::prelude! {}
    /// let fn_type = Typ::function(vec![Typ::int(), Typ::int()], Typ::int());
    /// assert!(fn_type.get_inputs().all(|ty| ty == &Typ::int()));
    /// ```
    pub fn get_inputs(&self) -> impl Iterator<Item = &Typ> + '_ {
        match self {
            Typ::Fn { inputs, .. } => inputs.iter(),
            _ => noErrorDesc!(),
        }
    }

    /// Conversion from FRP types to StateMachine types.
    ///
    /// Converts `signal T` into `T` and `event T` into `T?`.
    ///
    /// **NB:** this function panics on any other input.
    ///
    /// ```rust
    /// # grust_compiler_common::prelude! {}
    /// let s_type = Typ::signal(Typ::int());
    /// let e_type = Typ::event(Typ::bool());
    ///
    /// assert_eq!(s_type, Typ::signal(Typ::int()));
    /// assert_eq!(e_type, Typ::event(Typ::bool()));
    ///
    /// assert_eq!(s_type.convert(), Typ::int());
    /// assert_eq!(e_type.convert(), Typ::option(Typ::bool()));
    /// ```
    ///
    /// # Example
    ///
    /// In the example bellow, when calling the component `my_comp`, the integer signal `s` is
    /// converted into an integer `x` and the boolean event `e` is converted into an optional
    /// boolean `c`.
    ///
    /// ```gr
    /// import signal s: int;
    /// import event  e: bool;
    /// export signal res: int;
    ///
    /// service example {
    ///     res = my_comp(s, e);
    /// }
    ///
    /// component my_comp(int x, bool? c) -> (res: int) {
    ///     init res = 0;
    ///     res = when c then x else prev_res;
    ///     prev_res: int = last res;
    /// }
    /// ```
    pub fn convert(&self) -> Self {
        match self {
            Typ::Signal { ty, .. } => ty.as_ref().clone(),
            Typ::Event { ty, event_token } => Typ::Option {
                ty: ty.clone(),
                question_token: Token![?](event_token.span),
            },
            _ => noErrorDesc!(),
        }
    }
    /// Conversion from StateMachine types to FRP types.
    ///
    /// Converts `T` into `signal T` and `T?` into `event T`.
    ///
    /// **NB:** this function panics on `signal T` and `event T`.
    ///
    /// ```rust
    /// # grust_compiler_common::prelude! {}
    /// let s_type = Typ::signal(Typ::int());
    /// let e_type = Typ::event(Typ::bool());
    ///
    /// assert_eq!(s_type, Typ::signal(Typ::int()));
    /// assert_eq!(e_type, Typ::event(Typ::bool()));
    ///
    /// assert_eq!(s_type.convert(), Typ::int());
    /// assert_eq!(e_type.convert(), Typ::option(Typ::bool()));
    /// ```
    pub fn rev_convert(&self) -> Self {
        match self {
            Typ::Signal { .. } | Typ::Event { .. } => noErrorDesc!(),
            Typ::Option { ty, .. } => Typ::event((**ty).clone()),
            ty => Typ::signal(ty.clone()),
        }
    }

    pub fn is_event(&self) -> bool {
        matches!(self, Typ::Event { .. } | Typ::Option { .. })
    }

    pub fn is_polymorphic(&self) -> bool {
        use Typ::*;
        let mut stack = vec![];
        let mut current = self;

        'go_down: loop {
            match current {
                // early return, bypass the whole stack
                Polymorphism { .. } | NotDefinedYet(_) => return true,
                // leaves that don't require going down
                Integer(_)
                | Float(_)
                | Boolean(_)
                | Unit(_)
                | Enumeration { .. }
                | Structure { .. }
                | Any => (),
                // components we need to go down into
                Array { ty, .. } | Option { ty, .. } | Signal { ty, .. } | Event { ty, .. } => {
                    current = ty;
                    continue 'go_down;
                }
                Fn { inputs, output, .. } => {
                    for ty in inputs {
                        stack.push(ty);
                    }
                    current = output;
                    continue 'go_down;
                }
                Tuple { elements, .. } => {
                    let mut tys = elements.iter();
                    if let Some(ty) = tys.next() {
                        current = ty;
                        for ty in tys {
                            stack.push(ty);
                        }
                        continue 'go_down;
                    }
                    // otherwise just go up
                }
            }

            if let Some(next) = stack.pop() {
                current = next;
                continue 'go_down;
            } else {
                debug_assert!(stack.is_empty());
                return false;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn equality(input_types: Vec<Typ>, loc: Loc) -> Res<Typ> {
        let (lft, rgt) = check::arity::binary(loc, input_types)?;
        rgt.expect(loc, &lft)?;
        Ok(Typ::function(vec![lft, rgt], Typ::bool()))
    }

    #[test]
    fn should_apply_input_to_fn_type_when_compatible() {
        let mut errors = vec![];

        let input_types = vec![Typ::int()];
        let output_type = Typ::bool();
        let mut fn_type = Typ::function(input_types.clone(), output_type.clone());

        let application_result = fn_type
            .apply(input_types, Loc::test_dummy(), &mut errors)
            .unwrap();

        assert_eq!(application_result, output_type);
    }

    #[test]
    fn should_raise_error_when_incompatible_fn() {
        let mut errors = vec![];

        let input_types = vec![Typ::int()];
        let output_type = Typ::bool();
        let mut fn_type = Typ::function(input_types, output_type);

        fn_type
            .apply(vec![Typ::float()], Loc::test_dummy(), &mut errors)
            .unwrap_err();
    }

    #[test]
    fn should_return_non_polymorphic() {
        let mut errors = vec![];

        let mut polymorphic_type = Typ::poly(equality);

        let application_result = polymorphic_type
            .apply(vec![Typ::int(), Typ::int()], Loc::test_dummy(), &mut errors)
            .unwrap();

        let control = Typ::bool();

        assert_eq!(application_result, control);
    }

    #[test]
    fn should_raise_error_when_incompatible_polymorphic_type() {
        let mut errors = vec![];

        let mut polymorphic_type = Typ::poly(equality);

        let _ = polymorphic_type
            .apply(
                vec![Typ::int(), Typ::float()],
                Loc::test_dummy(),
                &mut errors,
            )
            .unwrap_err();
    }

    #[test]
    fn should_modify_polymorphic_type_to_non_polymorphic() {
        let mut errors = vec![];

        let mut polymorphic_type = Typ::poly(equality);

        let _ = polymorphic_type
            .apply(vec![Typ::int(), Typ::int()], Loc::test_dummy(), &mut errors)
            .unwrap();

        let control = Typ::function(vec![Typ::int(), Typ::int()], Typ::bool());

        assert_eq!(polymorphic_type, control);
    }

    #[test]
    fn should_return_inputs_from_fn_type() {
        let fn_type = Typ::function(vec![Typ::int(), Typ::int()], Typ::int());

        assert_eq!(
            fn_type.get_inputs().cloned().collect_vec(),
            vec![Typ::int(), Typ::int()]
        );
    }

    #[test]
    #[should_panic]
    fn should_panic_when_not_fn_type() {
        let not_fn_type = Typ::int();
        let _ = not_fn_type.get_inputs();
    }

    #[test]
    fn should_create_i64_from_ir2_integer() {
        let typ = Typ::int();
        let control = parse_quote! { i64 };
        let tokens: syn::Type = parse_quote!(#typ);
        assert_eq!(tokens, control)
    }

    #[test]
    fn should_create_f64_from_ir2_float() {
        let typ = Typ::float();
        let control = parse_quote! { f64 };
        let tokens: syn::Type = parse_quote!(#typ);
        assert_eq!(tokens, control)
    }

    #[test]
    fn should_create_bool_from_ir2_boolean() {
        let typ = Typ::bool();
        let control = parse_quote! { bool };
        let tokens: syn::Type = parse_quote!(#typ);
        assert_eq!(tokens, control)
    }

    #[test]
    fn should_create_unit_from_ir2_unit() {
        let typ = Typ::unit();
        let control = parse_quote! { () };

        let tokens: syn::Type = parse_quote!(#typ);
        assert_eq!(tokens, control)
    }

    #[test]
    fn should_create_structure_from_ir2_structure() {
        let typ = Typ::structure_str(Loc::test_id("Point"), 0);
        let control = parse_quote! { Point };

        let tokens: syn::Type = parse_quote!(#typ);
        assert_eq!(tokens, control)
    }

    #[test]
    fn should_create_enumeration_from_ir2_enumeration() {
        let typ = Typ::enumeration_str(Loc::test_id("Color"), 0);
        let control = parse_quote! { Color };

        let tokens: syn::Type = parse_quote!(#typ);
        assert_eq!(tokens, control)
    }

    #[test]
    fn should_create_array_from_ir2_array() {
        let typ = Typ::array(Typ::float(), 5);
        let control = parse_quote! { [f64; 5usize] };

        let tokens: syn::Type = parse_quote!(#typ);
        assert_eq!(tokens, control)
    }

    #[test]
    fn should_create_option_from_ir2_state_machine_event() {
        let typ = Typ::option(Typ::float());
        let control = parse_quote!(Option<f64>);
        let tokens: syn::Type = parse_quote!(#typ);
        assert_eq!(tokens, control)
    }

    #[test]
    fn should_create_closure_from_ir2_fn() {
        let typ = Typ::function(vec![Typ::int()], Typ::float());
        let control = parse_quote!(impl Fn(i64) -> f64);

        let tokens: syn::Type = parse_quote!(#typ);
        assert_eq!(tokens, control)
    }
}
