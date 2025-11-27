//! Contexts.

prelude! {}

pub struct Simple<'a> {
    pub ctx0: &'a mut Ctx,
    pub errors: &'a mut Vec<Error>,
}
impl ops::Deref for Simple<'_> {
    type Target = Ctx;
    fn deref(&self) -> &Self::Target {
        self.ctx0
    }
}
impl ops::DerefMut for Simple<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ctx0
    }
}
pub struct WithComp<'a> {
    pub ctx0: &'a mut Ctx,
    pub errors: &'a mut Vec<Error>,
    pub components: &'a HashMap<usize, &'a Component>,
    pub functions: &'a HashMap<usize, &'a Function>,
}
impl ops::Deref for WithComp<'_> {
    type Target = Ctx;
    fn deref(&self) -> &Self::Target {
        self.ctx0
    }
}
impl ops::DerefMut for WithComp<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ctx0
    }
}
pub struct WithLoc<'a> {
    pub loc: Loc,
    pub ctx0: &'a mut Ctx,
    pub errors: &'a mut Vec<Error>,
}
impl ops::Deref for WithLoc<'_> {
    type Target = Ctx;
    fn deref(&self) -> &Self::Target {
        self.ctx0
    }
}
impl ops::DerefMut for WithLoc<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ctx0
    }
}
pub struct PatLoc<'a> {
    pub pat: Option<&'a ir0::stmt::Pattern>,
    pub loc: Loc,
    pub ctx0: &'a mut Ctx,
    pub errors: &'a mut Vec<Error>,
}
impl ops::Deref for PatLoc<'_> {
    type Target = Ctx;
    fn deref(&self) -> &Self::Target {
        self.ctx0
    }
}
impl ops::DerefMut for PatLoc<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ctx0
    }
}
pub struct GenLoc<'a> {
    pub codegen: bool,
    pub loc: Loc,
    pub ctx0: &'a mut Ctx,
    pub errors: &'a mut Vec<Error>,
}
impl ops::Deref for GenLoc<'_> {
    type Target = Ctx;
    fn deref(&self) -> &Self::Target {
        self.ctx0
    }
}
impl ops::DerefMut for GenLoc<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ctx0
    }
}
impl<'a> Simple<'a> {
    pub fn new(ctx0: &'a mut Ctx, errors: &'a mut Vec<Error>) -> Self {
        Self { ctx0, errors }
    }
    pub fn add_loc(&mut self, loc: Loc) -> WithLoc<'_> {
        WithLoc::new(loc, self.ctx0, self.errors)
    }
    pub fn add_pat_loc<'b>(
        &'b mut self,
        pat: Option<&'b ir0::stmt::Pattern>,
        loc: impl Into<Loc>,
    ) -> PatLoc<'b> {
        PatLoc::new(pat, loc, self.ctx0, self.errors)
    }
    pub fn add_gen_loc(&mut self, codegen: bool, loc: impl Into<Loc>) -> GenLoc<'_> {
        GenLoc::new(codegen, loc, self.ctx0, self.errors)
    }
    pub fn add_components<'b>(
        &'b mut self,
        components: &'b HashMap<usize, &'b Component>,
        functions: &'a HashMap<usize, &'a Function>,
    ) -> WithComp<'b> {
        WithComp::new(self.ctx0, self.errors, components, functions)
    }
}
impl<'a> WithComp<'a> {
    pub fn new(
        ctx0: &'a mut Ctx,
        errors: &'a mut Vec<Error>,
        components: &'a HashMap<usize, &'a Component>,
        functions: &'a HashMap<usize, &'a Function>,
    ) -> Self {
        Self {
            ctx0,
            errors,
            components,
            functions,
        }
    }
}
impl<'a> WithLoc<'a> {
    pub fn new(loc: Loc, ctx0: &'a mut Ctx, errors: &'a mut Vec<Error>) -> Self {
        Self { loc, ctx0, errors }
    }
    pub fn rm_loc(&mut self) -> Simple<'_> {
        Simple::new(self.ctx0, self.errors)
    }
    pub fn add_pat<'b>(&'b mut self, pat: Option<&'b ir0::stmt::Pattern>) -> PatLoc<'b> {
        PatLoc::new(pat, self.loc, self.ctx0, self.errors)
    }
    pub fn add_gen(&mut self, codegen: bool) -> GenLoc<'_> {
        GenLoc::new(codegen, self.loc, self.ctx0, self.errors)
    }
}
impl<'a> PatLoc<'a> {
    pub fn new(
        pat: Option<&'a ir0::stmt::Pattern>,
        loc: impl Into<Loc>,
        ctx0: &'a mut Ctx,
        errors: &'a mut Vec<Error>,
    ) -> Self {
        Self {
            pat,
            loc: loc.into(),
            ctx0,
            errors,
        }
    }
    pub fn rm_pat(&mut self) -> WithLoc {
        WithLoc::new(self.loc, self.ctx0, self.errors)
    }
    pub fn rm_pat_loc(&mut self) -> Simple<'_> {
        Simple::new(self.ctx0, self.errors)
    }
    pub fn set_pat(
        &mut self,
        pat: Option<&'a ir0::stmt::Pattern>,
    ) -> Option<&'a ir0::stmt::Pattern> {
        std::mem::replace(&mut self.pat, pat)
    }
}
impl<'a> GenLoc<'a> {
    pub fn new(
        codegen: bool,
        loc: impl Into<Loc>,
        ctx0: &'a mut Ctx,
        errors: &'a mut Vec<Error>,
    ) -> Self {
        Self {
            codegen,
            loc: loc.into(),
            ctx0,
            errors,
        }
    }
    pub fn rm_loc(&mut self) -> WithLoc<'_> {
        WithLoc::new(self.loc, self.ctx0, self.errors)
    }
}

/// A flows context from where components will get their inputs.
#[derive(Debug, PartialEq, Default)]
pub struct Flows {
    pub elements: HashMap<Ident, Typ>,
}

impl Flows {
    pub fn add_element(&mut self, element_name: Ident, element_type: &Typ) {
        if let Some(other_ty) = self.elements.insert(element_name, element_type.clone()) {
            debug_assert!(other_ty.eq(element_type))
        }
    }
    pub fn contains_element(&self, element_name: &Ident) -> bool {
        self.elements.contains_key(element_name)
    }
}

impl ToTokens for Flows {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // sub-modules
        {
            let items = self.elements.iter().map(|(element_name, element_ty)| {
                let struct_name = element_name.to_camel();
                let name = element_name;
                let super_path = Ident::new("super", Span::call_site()).into();
                let ty = element_ty.to_prefix(&super_path);
                quote! {
                    #[derive(Clone, Copy, PartialEq, Default, Debug)]
                    pub struct #struct_name(#ty, bool);
                    impl #struct_name {
                        pub fn set(&mut self, #name: #ty) {
                            self.1 = self.0 != #name;
                            self.0 = #name;
                        }
                        pub fn get(&self) -> #ty { self.0 }
                        pub fn take(&mut self) -> #ty { std::mem::take(&mut self.0) }
                        pub fn is_new(&self) -> bool { self.1 }
                        pub fn reset(&mut self) { self.1 = false; }
                    }
                }
            });
            (quote! {
                mod ctx_ty { #(#items)* }
            })
            .to_tokens(tokens);
        }

        // `Context` structure type
        {
            let fields = self.elements.keys().map(|element_name| {
                let struct_name = element_name.to_camel();
                quote!(pub #element_name: ctx_ty::#struct_name)
            });
            (quote! {
                #[derive(Clone, Copy, PartialEq, Default, Debug)]
                pub struct Context { #(#fields),* }
            })
            .to_tokens(tokens);
        }

        // `Context` implementation
        {
            // `init` function
            let init_fun = quote! {
                fn init() -> Context {
                    Default::default()
                }
            };
            let reset_fun = {
                let stmts = self.elements.keys().map(|element_name| {
                    quote! {
                        self.#element_name.reset();
                    }
                });
                quote! {
                    fn reset(&mut self) { #(#stmts)* }
                }
            };
            (quote! {
                impl Context {
                    #init_fun
                    #reset_fun
                }
            })
            .to_tokens(tokens)
        }
    }
}

pub struct Full<'a, Event> {
    pub imports: &'a mut HashMap<usize, interface::FlowImport>,
    pub exports: &'a HashMap<usize, interface::FlowExport>,
    pub timings: &'a mut Vec<Event>,
    pub ctx0: &'a mut Ctx,
}
impl<E> ops::Deref for Full<'_, E> {
    type Target = Ctx;
    fn deref(&self) -> &Self::Target {
        self.ctx0
    }
}
impl<E> ops::DerefMut for Full<'_, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ctx0
    }
}

mk_new! { impl{'a, Event} Full<'a, Event> => new {
    imports: &'a mut HashMap<usize, interface::FlowImport>,
    exports: &'a HashMap<usize, interface::FlowExport>,
    timings: &'a mut Vec<Event>,
    ctx0: &'a mut Ctx,
} }
