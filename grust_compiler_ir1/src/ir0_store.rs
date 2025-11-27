prelude! { ir0::{Component, Colon} }

pub trait Ir0Store {
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<()>;
}

impl Ir0Store for Component {
    /// Store component's idents in symbol table.
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<()> {
        let loc = self.loc();
        let ctx = &mut ctx.add_loc(loc);

        ctx.local();

        // store input idents and get their ids
        let inputs = self
            .args
            .iter()
            .map(
                |Colon {
                     left: ident,
                     right: typ,
                     ..
                 }| {
                    let typ = typ.clone().into_ir1(ctx)?;
                    let id = ctx.ctx0.insert_ident(
                        ident.clone(),
                        Scope::Input,
                        Some(typ),
                        true,
                        ctx.errors,
                    )?;
                    Ok(id)
                },
            )
            .collect::<TRes<Vec<_>>>()?;

        // store outputs and get their ids
        let outputs = self
            .outs
            .iter()
            .map(
                |Colon {
                     left: ident,
                     right: typ,
                     ..
                 }| {
                    let typ = typ.clone().into_ir1(ctx)?;
                    let id = ctx.ctx0.insert_ident(
                        ident.clone(),
                        Scope::Output,
                        Some(typ),
                        true,
                        ctx.errors,
                    )?;
                    Ok((self.ident.clone(), id)) // TODO: ident.clone()
                },
            )
            .collect::<TRes<Vec<_>>>()?;

        // store locals and get their ids
        let locals = {
            let mut locals = HashMap::with_capacity(25);
            for equation in self.equations.iter() {
                equation.store_idents(false, &mut locals, &mut ctx.rm_loc())?;
            }
            locals.shrink_to_fit();
            locals
        };

        // store inits and get their ids
        let inits = {
            let mut inits = HashMap::with_capacity(5);
            for equation in self.equations.iter() {
                equation.store_inits(&mut inits, &mut ctx.rm_loc())?;
            }
            inits.shrink_to_fit();
            inits
        };

        ctx.global();

        let _ = ctx.ctx0.insert_comp(
            self.ident.clone(),
            (inputs, outputs),
            Some((locals, inits)),
            None,
            self.weight,
            ctx.errors,
        )?;

        Ok(())
    }
}

pub trait Ir0StoreIdents {
    /// Creates identifiers for the equation (depending on the config `store_outputs`)
    ///
    /// # Example
    ///
    /// ```grust
    /// match e {
    ///     pat1 => { let a: int = e_a; let b: float = e_b; },
    ///     pat2 => { let (a: int, b: float) = e_ab; },
    /// }
    /// ```
    ///
    /// With the above equations, the algorithm insert in the `idents` map
    /// [ a -> id_a ] and [ b -> id_b ].
    fn store_idents(
        &self,
        store_outputs: bool,
        idents: &mut HashMap<Ident, usize>,
        ctx: &mut ctx::Simple,
    ) -> TRes<()>;

    fn store_inits(&self, inits: &mut HashMap<Ident, usize>, ctx: &mut ctx::Simple) -> TRes<()>;

    /// Collects the identifiers of the equation.
    fn get_idents(&self, idents: &mut HashMap<Ident, ir0::stmt::Pattern>);

    /// Collects the initializations.
    fn get_inits(&self, inits: &mut HashMap<Ident, ir0::stmt::Pattern>);
}

pub trait Ir0StoreInit: Sized {
    fn store_inits(&self, inits: &mut HashMap<Ident, usize>, ctx: &mut ctx::Simple) -> TRes<()>;

    fn get_inits(&self, inits: &mut HashMap<Ident, ir0::stmt::Pattern>);
}

mod equation {
    prelude! { ir0::equation::* }

    impl Ir0StoreIdents for Eq {
        fn store_idents(
            &self,
            store_outputs: bool,
            idents: &mut HashMap<Ident, usize>,
            ctx: &mut ctx::Simple,
        ) -> TRes<()> {
            match self {
                // when output definitions should be stored
                Eq::OutputDef(instantiation) if store_outputs => instantiation
                    .pattern
                    .store(ctx)
                    .map(|new_idents| idents.extend(new_idents)),
                // when output definitions are already stored (as component outputs)
                Eq::OutputDef(_) => Ok(()),
                Eq::LocalDef(declaration) => declaration
                    .typed_pattern
                    .store(ctx)
                    .map(|new_idents| idents.extend(new_idents)),
                Eq::MatchEq(MatchEq { arms, .. }) => {
                    let Arm { equations, .. } = arms.first().unwrap();
                    for eq in equations.iter() {
                        eq.store_idents(store_outputs, idents, ctx)?;
                    }
                    Ok(())
                }
            }
        }

        fn store_inits(
            &self,
            _inits: &mut HashMap<Ident, usize>,
            _ctx: &mut ctx::Simple,
        ) -> TRes<()> {
            Ok(())
        }

        fn get_idents(&self, idents: &mut HashMap<Ident, ir0::stmt::Pattern>) {
            match self {
                Eq::OutputDef(instantiation) => idents.extend(instantiation.pattern.get_idents()),
                Eq::LocalDef(declaration) => idents.extend(declaration.typed_pattern.get_idents()),
                Eq::MatchEq(MatchEq { arms, .. }) => {
                    let Arm { equations, .. } = arms.first().unwrap();
                    for eq in equations {
                        eq.get_idents(idents);
                    }
                }
            }
        }

        fn get_inits(&self, _inits: &mut HashMap<Ident, ir0::stmt::Pattern>) {}
    }

    impl Ir0StoreInit for InitArmWhen {
        fn store_inits(
            &self,
            inits: &mut HashMap<Ident, usize>,
            ctx: &mut ctx::Simple,
        ) -> TRes<()> {
            for init in &self.equations {
                let idents = init.pattern.store_inits(ctx)?;
                inits.extend(idents);
            }
            Ok(())
        }

        fn get_inits(&self, inits: &mut HashMap<Ident, ir0::stmt::Pattern>) {
            for init in &self.equations {
                let idents = init.pattern.get_idents();
                inits.extend(idents);
            }
        }
    }

    impl Ir0StoreIdents for ReactEq {
        fn store_idents(
            &self,
            store_outputs: bool,
            idents: &mut HashMap<Ident, usize>,
            ctx: &mut ctx::Simple,
        ) -> TRes<()> {
            match self {
                // output definitions should be stored
                ReactEq::OutputDef(instantiation) if store_outputs => instantiation
                    .pattern
                    .store(ctx)
                    .map(|new_idents| idents.extend(new_idents)),
                // when output definitions are already stored (as component's outputs)
                ReactEq::OutputDef(_) | ReactEq::Init(_) | ReactEq::Log(_) => Ok(()),
                ReactEq::LocalDef(declaration) => declaration
                    .typed_pattern
                    .store(ctx)
                    .map(|new_idents| idents.extend(new_idents)),
                ReactEq::MatchEq(MatchEq { arms, .. }) => {
                    let Arm { equations, .. } = arms.first().unwrap();
                    for eq in equations.iter() {
                        eq.store_idents(store_outputs, idents, ctx)?;
                    }
                    Ok(())
                }
                ReactEq::WhenEq(WhenEq { arms, .. }) => {
                    // we want to collect every identifier, but events might be declared in only one
                    // branch then, it is needed to explore all branches
                    let mut when_idents = HashMap::new();
                    let mut add_idents = |equations: &[Eq]| {
                        // some flows are defined in multiple branches so we don't want them to trigger
                        // the *duplicated definition* error.
                        ctx.local();
                        for eq in equations {
                            eq.store_idents(store_outputs, &mut when_idents, ctx)?;
                        }
                        ctx.global();
                        Ok(())
                    };
                    for EventArmWhen { equations, .. } in arms {
                        add_idents(equations)?
                    }
                    // put the identifiers back in context
                    for (k, v) in when_idents.into_iter() {
                        let loc = k.loc();
                        if let std::collections::hash_map::Entry::Vacant(e) = idents.entry(k) {
                            e.insert(v);
                            ctx.ctx0.put_back_in_context(v, false, loc, ctx.errors)?;
                        }
                    }
                    Ok(())
                }
            }
        }

        fn store_inits(
            &self,
            inits: &mut HashMap<Ident, usize>,
            ctx: &mut ctx::Simple,
        ) -> TRes<()> {
            match self {
                ReactEq::Init(init) => init
                    .pattern
                    .store_inits(ctx)
                    .map(|idents| inits.extend(idents)),
                ReactEq::OutputDef(instantiation) => {
                    if let ir0::stream::ReactExpr::WhenExpr(ir0::stream::WhenExpr {
                        init: Some(_),
                        ..
                    }) = instantiation.expr
                    {
                        instantiation
                            .pattern
                            .store_inits(ctx)
                            .map(|idents| inits.extend(idents))?;
                    }
                    Ok(())
                }
                ReactEq::LocalDef(declaration) => {
                    if let ir0::stream::ReactExpr::WhenExpr(ir0::stream::WhenExpr {
                        init: Some(_),
                        ..
                    }) = declaration.expr
                    {
                        declaration
                            .typed_pattern
                            .store_inits(ctx)
                            .map(|idents| inits.extend(idents))?;
                    }
                    Ok(())
                }
                ReactEq::MatchEq(_) | ReactEq::Log(_) => Ok(()),
                ReactEq::WhenEq(WhenEq { init, .. }) => {
                    // store initializations
                    if let Some(init) = init {
                        init.store_inits(inits, ctx)?;
                    }
                    Ok(())
                }
            }
        }

        fn get_idents(&self, idents: &mut HashMap<Ident, ir0::stmt::Pattern>) {
            match self {
                ReactEq::OutputDef(instantiation) => {
                    idents.extend(instantiation.pattern.get_idents())
                }
                ReactEq::LocalDef(declaration) => {
                    idents.extend(declaration.typed_pattern.get_idents())
                }
                ReactEq::MatchEq(MatchEq { arms, .. }) => {
                    let Arm { equations, .. } = arms.first().unwrap();
                    for eq in equations {
                        eq.get_idents(idents);
                    }
                }
                ReactEq::WhenEq(WhenEq { arms, .. }) => {
                    let mut add_idents = |equations: &[Eq]| {
                        // we want to collect every identifier, but events might be declared in only
                        // one branch then, it is needed to explore all branches
                        for eq in equations {
                            eq.get_idents(idents);
                        }
                    };
                    for EventArmWhen { equations, .. } in arms {
                        add_idents(equations);
                    }
                }
                ReactEq::Init(_) | ReactEq::Log(_) => (),
            }
        }

        fn get_inits(&self, inits: &mut HashMap<Ident, ir0::stmt::Pattern>) {
            match self {
                ReactEq::OutputDef(_)
                | ReactEq::LocalDef(_)
                | ReactEq::MatchEq(_)
                | ReactEq::Log(_) => (),
                ReactEq::WhenEq(WhenEq { init, .. }) => {
                    if let Some(init) = init {
                        init.get_inits(inits)
                    }
                }
                ReactEq::Init(init) => {
                    inits.extend(init.pattern.get_idents());
                }
            }
        }
    }
}

impl Ir0Store for Ast {
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<()> {
        // store type definition first, so they can be used after
        self.items
            .iter()
            .filter_map(|item| match item {
                ir0::Item::Typedef(typedef) => Some(typedef),
                _ => None,
            })
            .map(|typedef| typedef.store(ctx))
            .collect::<TRes<Vec<_>>>()?;

        self.items
            .iter()
            .map(|item| match item {
                ir0::Item::Component(component) => component.store(ctx),
                ir0::Item::Function(function) => function.store(ctx),
                ir0::Item::ExtFun(extfun) => extfun.store(ctx),
                ir0::Item::ExtComp(extcomp) => extcomp.store(ctx),
                ir0::Item::Const(const_decl) => const_decl.store(ctx),
                ir0::Item::Typedef(_) // already stored
                | ir0::Item::Service(_)
                | ir0::Item::Import(_)
                | ir0::Item::Export(_) => Ok(()),
            })
            .collect::<TRes<Vec<_>>>()?;
        Ok(())
    }
}

impl Ir0Store for ir0::Function {
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<()> {
        let loc = self.loc();
        let ctx = &mut ctx.add_loc(loc);
        ctx.local();

        let inputs = self
            .args
            .iter()
            .map(
                |ir0::Colon {
                     left: ident,
                     right: typ,
                     ..
                 }| {
                    let typ = typ.clone().into_ir1(ctx)?;
                    let id =
                        ctx.ctx0
                            .insert_local_ident(ident.clone(), Some(typ), true, ctx.errors)?;
                    Ok(id)
                },
            )
            .collect::<TRes<Vec<_>>>()?;

        ctx.global();

        let _ =
            ctx.ctx0
                .insert_function(self.ident.clone(), inputs, None, self.weight, ctx.errors)?;

        Ok(())
    }
}

impl Ir0Store for ir0::ExtFunDecl {
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<()> {
        let loc = self.loc();
        let ctx = &mut ctx.add_loc(loc);
        ctx.local();

        let inputs = self
            .args
            .iter()
            .map(
                |ir0::Colon {
                     left: ident,
                     right: typ,
                     ..
                 }| {
                    let typ = typ.clone().into_ir1(ctx)?;
                    let id =
                        ctx.ctx0
                            .insert_local_ident(ident.clone(), Some(typ), true, ctx.errors)?;
                    Ok(id)
                },
            )
            .collect::<TRes<Vec<_>>>()?;

        ctx.global();

        let _ = ctx.ctx0.insert_function(
            self.ident.clone(),
            inputs,
            Some(self.path.clone()),
            self.weight,
            ctx.errors,
        )?;

        Ok(())
    }
}

impl Ir0Store for ir0::ExtCompDecl {
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<()> {
        let loc = self.loc();
        let ctx = &mut ctx.add_loc(loc);
        ctx.local();

        let inputs = self
            .args
            .iter()
            .map(
                |ir0::Colon {
                     left: ident,
                     right: typ,
                     ..
                 }| {
                    let typ = typ.clone().into_ir1(ctx)?;
                    let id =
                        ctx.ctx0
                            .insert_local_ident(ident.clone(), Some(typ), true, ctx.errors)?;
                    Ok(id)
                },
            )
            .collect::<TRes<Vec<_>>>()?;

        let outputs = self
            .outs
            .iter()
            .map(
                |ir0::Colon {
                     left: ident,
                     right: typ,
                     ..
                 }| {
                    let typ = typ.clone().into_ir1(ctx)?;
                    let id =
                        ctx.ctx0
                            .insert_local_ident(ident.clone(), Some(typ), true, ctx.errors)?;
                    Ok((self.ident.clone(), id))
                },
            )
            .collect::<TRes<Vec<_>>>()?;

        ctx.global();

        let _ = ctx.ctx0.insert_comp(
            self.ident.clone(),
            (inputs, outputs),
            None,
            Some(self.path.clone()),
            self.weight,
            ctx.errors,
        )?;
        // let _ = ctx.ctx0.insert_comp(
        //     self.ident.clone(),
        //     inputs,
        //     outputs,
        //     Some(locals),
        //     Some(inits),
        //     None,
        //     self.weight,
        //     ctx.errors,
        // )?;

        Ok(())
    }
}

impl Ir0Store for ir0::ConstDecl {
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<()> {
        let loc = self.loc();
        let ctx = &mut ctx.add_loc(loc);

        let _id = ctx.ctx0.insert_constant(
            self.ident.clone(),
            self.ty.clone(),
            self.value.clone(),
            ctx.errors,
        )?;

        Ok(())
    }
}

pub trait Ir0StorePattern: Sized {
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<()>;

    fn get_idents(&self) -> TRes<Vec<(Ident, Self)>>;
}

mod expr_pattern {
    prelude! {
        ir0::expr::{PatStructure, PatTuple, Pattern},
    }

    impl Ir0StorePattern for Pattern {
        fn store(&self, ctx: &mut ctx::Simple) -> TRes<()> {
            match self {
                Pattern::Identifier(name) => ctx
                    .ctx0
                    .insert_ident(name.clone(), Scope::VeryLocal, None, true, ctx.errors)
                    .map(|_| ()),
                Pattern::Tuple(PatTuple { elements, .. }) => {
                    elements.iter().try_for_each(|pattern| pattern.store(ctx))
                }
                Pattern::Structure(PatStructure { fields, .. }) => {
                    fields.iter().try_for_each(|(field, optional_pattern)| {
                        if let Some(pattern) = optional_pattern {
                            pattern.store(ctx)
                        } else {
                            ctx.ctx0
                                .insert_local_ident(field.clone(), None, true, ctx.errors)
                                .map(|_| ())
                        }
                    })
                }
                Pattern::Constant(_) | Pattern::Enumeration(_) | Pattern::Default(_) => Ok(()),
            }
        }

        fn get_idents(&self) -> TRes<Vec<(Ident, Pattern)>> {
            match self {
                Pattern::Identifier(name) => Ok(vec![(name.clone(), self.clone())]),
                Pattern::Tuple(PatTuple { elements, .. }) => Ok(elements
                    .iter()
                    .map(|pattern| pattern.get_idents())
                    .collect::<TRes<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect()),
                Pattern::Structure(PatStructure { fields, .. }) => Ok(fields
                    .iter()
                    .map(|(field, optional_pattern)| {
                        if let Some(pattern) = optional_pattern {
                            pattern.get_idents()
                        } else {
                            Ok(vec![(field.clone(), Pattern::ident(field.clone()))])
                        }
                    })
                    .collect::<TRes<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect()),
                Pattern::Constant(_) | Pattern::Enumeration(_) | Pattern::Default(_) => Ok(vec![]),
            }
        }
    }
}

pub trait Ir0StoreStmtPattern: Sized {
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<Vec<(Ident, usize)>>;

    fn store_inits(&self, ctx: &mut ctx::Simple) -> TRes<Vec<(Ident, usize)>>;

    fn get_idents(&self) -> Vec<(Ident, Self)>;
}

mod stmt_pattern {
    prelude! {
        ir0::stmt::{Pattern, Tuple, Typed},
    }

    impl Ir0StoreStmtPattern for Pattern {
        fn store(&self, ctx: &mut ctx::Simple) -> TRes<Vec<(Ident, usize)>> {
            let loc = self.loc();

            match self {
                Pattern::Identifier(ident) => {
                    let id = ctx.ctx0.get_identifier_id(ident, false, ctx.errors)?;
                    // outputs should be already typed
                    let typ = ctx.get_typ(id).clone();
                    let id =
                        ctx.ctx0
                            .insert_local_ident(ident.clone(), Some(typ), true, ctx.errors)?;
                    Ok(vec![(ident.clone(), id)])
                }
                Pattern::Typed(Typed { ident, typ, .. }) => {
                    let typ = typ.clone().into_ir1(&mut ctx.add_loc(loc))?;
                    let id =
                        ctx.ctx0
                            .insert_local_ident(ident.clone(), Some(typ), true, ctx.errors)?;
                    Ok(vec![(ident.clone(), id)])
                }
                Pattern::Tuple(Tuple { elements, .. }) => Ok(elements
                    .iter()
                    .map(|pattern| pattern.store(ctx))
                    .collect::<TRes<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect()),
            }
        }

        fn store_inits(&self, ctx: &mut ctx::Simple) -> TRes<Vec<(Ident, usize)>> {
            let loc = self.loc();

            match self {
                Pattern::Identifier(ident) => {
                    let id = ctx.ctx0.get_identifier_id(ident, false, ctx.errors)?;
                    // outputs should be already typed
                    let typ = ctx.get_typ(id).clone();
                    let id = ctx
                        .ctx0
                        .insert_init(ident.clone(), Some(typ), true, ctx.errors)?;
                    Ok(vec![(ident.clone(), id)])
                }
                Pattern::Typed(Typed { ident, typ, .. }) => {
                    let typ = typ.clone().into_ir1(&mut ctx.add_loc(loc))?;
                    let id = ctx
                        .ctx0
                        .insert_init(ident.clone(), Some(typ), true, ctx.errors)?;
                    Ok(vec![(ident.clone(), id)])
                }
                Pattern::Tuple(Tuple { elements, .. }) => Ok(elements
                    .iter()
                    .map(|pattern| pattern.store_inits(ctx))
                    .collect::<TRes<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect()),
            }
        }

        fn get_idents(&self) -> Vec<(Ident, Pattern)> {
            match self {
                Pattern::Identifier(ident) | Pattern::Typed(Typed { ident, .. }) => {
                    vec![(ident.clone(), self.clone())]
                }
                Pattern::Tuple(Tuple { elements, .. }) => elements
                    .iter()
                    .flat_map(|pattern| pattern.get_idents())
                    .collect(),
            }
        }
    }
}

impl Ir0Store for ir0::Typedef {
    /// Store typedef's identifiers in symbol table.
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<()> {
        let loc = self.loc();
        let ctx = &mut ctx.add_loc(loc);

        match self {
            ir0::Typedef::Structure { ident, fields, .. } => {
                ctx.local();

                let field_ids = fields
                    .iter()
                    .map(
                        |ir0::Colon {
                             left: ident,
                             right: typing,
                             ..
                         }| {
                            let typing = typing.clone().into_ir1(ctx)?;
                            let field_id = ctx.ctx0.insert_local_ident(
                                ident.clone(),
                                Some(typing),
                                true,
                                ctx.errors,
                            )?;
                            Ok(field_id)
                        },
                    )
                    .collect::<TRes<Vec<_>>>()?;

                ctx.global();

                let _ =
                    ctx.ctx0
                        .insert_struct(ident.clone(), field_ids.clone(), false, ctx.errors)?;
            }
            ir0::Typedef::Enumeration {
                ident, elements, ..
            } => {
                let element_ids = elements
                    .iter()
                    .map(|element_ident| {
                        let element_id = ctx.ctx0.insert_enum_elem(
                            element_ident.clone(),
                            ident.clone(),
                            false,
                            ctx.errors,
                        )?;
                        Ok(element_id)
                    })
                    .collect::<TRes<Vec<_>>>()?;

                let _ =
                    ctx.ctx0
                        .insert_enum(ident.clone(), element_ids.clone(), false, ctx.errors)?;
            }
            ir0::Typedef::Array {
                ident,
                size,
                array_type,
                ..
            } => {
                let typing = array_type.clone().into_ir1(ctx)?;
                let size = size.base10_parse().unwrap();
                let _ =
                    ctx.ctx0
                        .insert_array(ident.clone(), Some(typing), size, false, ctx.errors)?;
            }
        }

        Ok(())
    }
}

pub trait Ir0StoreEventPattern {
    /// Accumulates in `events_indices` the indices of events in the matched tuple.
    fn place_events(
        &self,
        events_indices: &mut HashMap<usize, usize>,
        idx: &mut usize,
        ctx: &mut ctx::Simple,
    ) -> TRes<()>;
    /// Stores the identifiers defined by the pattern in the symbol table.
    fn store(&self, ctx: &mut ctx::Simple) -> TRes<()>;
}

mod event_pattern {
    prelude! {
        ir0::equation::EventPattern,
    }

    impl Ir0StoreEventPattern for EventPattern {
        fn place_events(
            &self,
            events_indices: &mut HashMap<usize, usize>,
            idx: &mut usize,
            ctx: &mut ctx::Simple,
        ) -> TRes<()> {
            match self {
                EventPattern::Tuple(tuple) => tuple
                    .patterns
                    .iter()
                    .try_for_each(|pattern| pattern.place_events(events_indices, idx, ctx)),
                EventPattern::Let(pattern) => {
                    let event_id = ctx
                        .ctx0
                        .get_identifier_id(&pattern.event, false, ctx.errors)?;
                    let _ = events_indices.entry(event_id).or_insert_with(|| {
                        let v = *idx;
                        *idx += 1;
                        v
                    });
                    Ok(())
                }
                EventPattern::RisingEdge(_) => Ok(()),
            }
        }

        fn store(&self, ctx: &mut ctx::Simple) -> TRes<()> {
            match self {
                EventPattern::Tuple(tuple) => tuple
                    .patterns
                    .iter()
                    .try_for_each(|pattern| pattern.store(ctx)),
                EventPattern::Let(pattern) => pattern.pattern.store(ctx).map(|_| ()),
                EventPattern::RisingEdge(_) => Ok(()),
            }
        }
    }
}
