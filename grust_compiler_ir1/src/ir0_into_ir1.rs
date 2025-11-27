prelude! { ir0::symbol::SymbolKind }

/// [ir0] transformation into [ir1], implemented by [ir0] types.
pub trait Ir0IntoIr1<Ctx> {
    /// Corresponding [ir1] construct.
    type Ir1;

    /// Transforms [ir0] into [ir1] and check identifiers good use.
    fn into_ir1(self, ctx: &mut Ctx) -> TRes<Self::Ir1>;
}

impl Ir0IntoIr1<ctx::Simple<'_>> for Ast {
    type Ir1 = File;

    fn into_ir1(self, ctx: &mut ctx::Simple) -> TRes<Self::Ir1> {
        // store elements in symbol table
        self.store(ctx)?;

        let (mut typedefs, mut functions, mut components, mut imports, mut exports, mut services) = (
            Vec::with_capacity(20),
            Vec::with_capacity(20),
            Vec::with_capacity(20),
            HashMap::with_capacity(20),
            HashMap::with_capacity(20),
            Vec::with_capacity(20),
        );

        for item in self.items {
            match item {
                ir0::Item::Component(component) => components.push(component.into_ir1(ctx)?),
                ir0::Item::Function(function) => functions.push(function.into_ir1(ctx)?),
                ir0::Item::Typedef(typedef) => typedefs.push(typedef.into_ir1(ctx)?),
                ir0::Item::Service(service) => services.push(service.into_ir1(ctx)?),
                ir0::Item::Import(import) => {
                    let ir1 = import.into_ir1(ctx)?;
                    let id = ctx.get_fresh_id();
                    let _prev = imports.insert(id, ir1);
                    debug_assert!(_prev.is_none());
                }
                ir0::Item::Export(export) => {
                    let ir1 = export.into_ir1(ctx)?;
                    let id = ctx.get_fresh_id();
                    let _prev = exports.insert(id, ir1);
                    debug_assert!(_prev.is_none());
                }
                ir0::Item::ExtFun(ext) => functions.push(ext.into_ir1(ctx)?),
                ir0::Item::ExtComp(extcomp) => components.push(extcomp.into_ir1(ctx)?),
                ir0::Item::Const(_) => (),
            }
        }

        let interface = Interface {
            services,
            imports,
            exports,
        };

        Ok(File {
            typedefs,
            functions,
            components,
            interface,
            loc: Loc::nu_call_site(),
        })
    }
}

mod interface_impl {
    prelude! {
        ir0::interface::{
            TimeRange, FlowDeclaration, FlowExport, FlowImport,
            FlowInstantiation, FlowKind, FlowPattern, FlowStatement, Service,
        },
            interface::{
                FlowDeclaration as Ir1FlowDeclaration, FlowExport as Ir1FlowExport,
                FlowImport as Ir1FlowImport, FlowInstantiation as Ir1FlowInstantiation,
                FlowStatement as Ir1FlowStatement,
            },
    }

    fn into_u64(time: Either<syn::LitInt, Ident>, ctx: &mut ctx::Simple<'_>) -> TRes<u64> {
        match time {
            Either::Left(lit) => Ok(lit.base10_parse().unwrap()),
            Either::Right(ident) => match ctx.ctx0.get_const(&ident, ctx.errors)? {
                ir0::Expr::Constant(Constant::Integer(lit)) => Ok(lit.base10_parse().unwrap()),
                _ => Err(error!(@ident.loc() => ErrorKind::msg("integer literal expected"))),
            }
            .dewrap(ctx.errors),
        }
    }

    impl<'a> Ir0IntoIr1<ctx::Simple<'a>> for TimeRange {
        type Ir1 = (u64, u64);

        fn into_ir1(self, ctx: &mut ctx::Simple<'a>) -> TRes<Self::Ir1> {
            let min = into_u64(self.min, ctx)?;
            let max = into_u64(self.max, ctx)?;
            Ok((min, max))
        }
    }

    impl<'a> Ir0IntoIr1<ctx::Simple<'a>> for Service {
        type Ir1 = ir1::Service;

        fn into_ir1(self, ctx: &mut ir1::ctx::Simple<'a>) -> TRes<Self::Ir1> {
            let id = ctx.ctx0.insert_service(self.ident, true, ctx.errors)?;

            let time_range = self.time_range.map(|tr| tr.into_ir1(ctx)).transpose()?;

            ctx.local();
            let statements = self
                .flow_statements
                .into_iter()
                .map(|flow_statement| {
                    flow_statement
                        .into_ir1(ctx)
                        .map(|res| (ctx.get_fresh_id(), res))
                })
                .collect::<TRes<HashMap<_, _>>>()?;
            let graph = Default::default();
            ctx.global();

            Ok(ir1::Service {
                id,
                time_range,
                statements,
                graph,
            })
        }
    }

    impl<'a> Ir0IntoIr1<ir1::ctx::Simple<'a>> for FlowImport {
        type Ir1 = Ir1FlowImport;

        fn into_ir1(mut self, ctx: &mut ir1::ctx::Simple<'a>) -> TRes<Self::Ir1> {
            let loc = self.loc();

            let last = self.typed_path.left.segments.pop().unwrap().into_value();
            assert!(last.arguments.is_none());
            let path = self.typed_path.left;
            let flow_type = {
                let inner = self.typed_path.right.into_ir1(&mut ctx.add_loc(loc))?;
                match self.kind {
                    FlowKind::Signal(_) => Typ::signal(inner),
                    FlowKind::Event(_) => Typ::event(inner),
                }
            };
            let id = ctx.ctx0.insert_flow(
                last.ident,
                Some(path.clone()),
                self.kind,
                flow_type.clone(),
                true,
                ctx.errors,
            )?;

            Ok(Ir1FlowImport {
                token: Either::Left(self.import_token),
                id,
                path,
                colon_token: self.typed_path.colon,
                flow_type,
                semi_token: self.semi_token,
            })
        }
    }

    impl<'a> Ir0IntoIr1<ir1::ctx::Simple<'a>> for FlowExport {
        type Ir1 = Ir1FlowExport;

        fn into_ir1(mut self, ctx: &mut ir1::ctx::Simple<'a>) -> TRes<Self::Ir1> {
            let loc = self.loc();

            let last = self.typed_path.left.segments.pop().unwrap().into_value();
            assert!(last.arguments.is_none());
            let path = self.typed_path.left;
            let flow_type = {
                let inner = self.typed_path.right.into_ir1(&mut ctx.add_loc(loc))?;
                match self.kind {
                    FlowKind::Signal(_) => Typ::signal(inner),
                    FlowKind::Event(_) => Typ::event(inner),
                }
            };
            let id = ctx.ctx0.insert_flow(
                last.ident,
                Some(path.clone()),
                self.kind,
                flow_type.clone(),
                true,
                ctx.errors,
            )?;

            Ok(Ir1FlowExport {
                export_token: self.export_token,
                id,
                path,
                colon_token: self.typed_path.colon,
                flow_type,
                semi_token: self.semi_token,
            })
        }
    }

    impl<'a> Ir0IntoIr1<ir1::ctx::Simple<'a>> for FlowStatement {
        type Ir1 = Ir1FlowStatement;

        fn into_ir1(self, ctx: &mut ir1::ctx::Simple<'a>) -> TRes<Self::Ir1> {
            let loc = self.loc();
            match self {
                FlowStatement::Declaration(FlowDeclaration {
                    let_token,
                    typed_pattern,
                    eq_token,
                    expr,
                    semi_token,
                }) => {
                    let pattern = typed_pattern.into_ir1(ctx)?;
                    let expr = expr.into_ir1(&mut ctx.add_loc(loc))?;

                    Ok(Ir1FlowStatement::Declaration(Ir1FlowDeclaration {
                        let_token,
                        pattern,
                        eq_token,
                        expr,
                        semi_token,
                    }))
                }
                FlowStatement::Instantiation(FlowInstantiation {
                    pattern,
                    eq_token,
                    expr,
                    semi_token,
                }) => {
                    // transform pattern and check its identifiers exist
                    let pattern = pattern.into_ir1(ctx)?;
                    // transform the expression
                    let expr = expr.into_ir1(&mut ctx.add_loc(loc))?;

                    Ok(Ir1FlowStatement::Instantiation(Ir1FlowInstantiation {
                        pattern,
                        eq_token,
                        expr,
                        semi_token,
                    }))
                }
            }
        }
    }

    impl<'a> Ir0IntoIr1<ir1::ctx::Simple<'a>> for FlowPattern {
        type Ir1 = ir1::stmt::Pattern;

        fn into_ir1(self, ctx: &mut ir1::ctx::Simple<'a>) -> TRes<Self::Ir1> {
            let loc = self.loc();

            match self {
                FlowPattern::Single { ident } => {
                    let id = ctx.ctx0.get_flow_id(&ident, false, ctx.errors)?;
                    let typ = ctx.get_typ(id);

                    Ok(ir1::stmt::Pattern {
                        kind: ir1::stmt::Kind::Identifier { id },
                        typ: Some(typ.clone()),
                        loc,
                    })
                }
                FlowPattern::SingleTyped {
                    kind, ident, ty, ..
                } => {
                    let inner = ty.into_ir1(&mut ctx.add_loc(loc))?;
                    let flow_typ = match kind {
                        FlowKind::Signal(_) => Typ::signal(inner),
                        FlowKind::Event(_) => Typ::event(inner),
                    };
                    let id = ctx.ctx0.insert_flow(
                        ident.clone(),
                        None,
                        kind,
                        flow_typ.clone(),
                        true,
                        ctx.errors,
                    )?;

                    Ok(ir1::stmt::Pattern {
                        kind: ir1::stmt::Kind::Typed {
                            id,
                            typ: flow_typ.clone(),
                        },
                        typ: Some(flow_typ),
                        loc,
                    })
                }
                FlowPattern::Tuple { patterns, .. } => {
                    let (mut elements, mut types) = (
                        Vec::with_capacity(patterns.len()),
                        Vec::with_capacity(patterns.len()),
                    );
                    for pat in patterns {
                        let loc = pat.loc();
                        let elem = pat.into_ir1(ctx)?;
                        let typ = elem
                            .typ
                            .as_ref()
                            .ok_or_else(lerror!(@loc =>
                                "[internal] failed to retrieve type of pattern"
                            ))
                            .dewrap(ctx.errors)?
                            .clone();
                        types.push(typ);
                        elements.push(elem);
                    }
                    let typ = if types.len() == 1 {
                        types.pop().unwrap()
                    } else {
                        Typ::tuple(types)
                    };
                    Ok(ir1::stmt::Pattern {
                        kind: ir1::stmt::Kind::Tuple { elements },
                        typ: Some(typ),
                        loc,
                    })
                }
            }
        }
    }
    mod flow_expr_impl {
        prelude! {
            ir0::interface::{
                FlowExpression, Call, OnChange, Merge,
                Scan, Throttle, Timeout, Time, Persist,
                Period, Sample, SampleOn, ScanOn,
            },
            ir1::flow,
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for Sample {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::sample(
                    self.expr.into_ir1(ctx)?,
                    super::into_u64(self.period_ms, &mut ctx.rm_loc())?,
                ))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for Scan {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::scan(
                    self.expr.into_ir1(ctx)?,
                    super::into_u64(self.period_ms, &mut ctx.rm_loc())?,
                ))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for Timeout {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::timeout(
                    self.expr.into_ir1(ctx)?,
                    super::into_u64(self.deadline, &mut ctx.rm_loc())?,
                ))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for Throttle {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::throttle(
                    self.expr.into_ir1(ctx)?,
                    self.delta,
                ))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for OnChange {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::on_change(self.expr.into_ir1(ctx)?))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for Persist {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::persist(self.expr.into_ir1(ctx)?))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for Merge {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::merge(
                    self.expr_1.into_ir1(ctx)?,
                    self.expr_2.into_ir1(ctx)?,
                ))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for Time {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, _ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::time(self.time_token.span.into()))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for Period {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::period(super::into_u64(
                    self.period_ms,
                    &mut ctx.rm_loc(),
                )?))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for SampleOn {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::sample_on(
                    self.expr.into_ir1(ctx)?,
                    self.event.into_ir1(ctx)?,
                ))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for ScanOn {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                Ok(ir1::flow::Kind::scan_on(
                    self.expr.into_ir1(ctx)?,
                    self.event.into_ir1(ctx)?,
                ))
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for Call {
            type Ir1 = ir1::flow::Kind;

            /// Transforms AST into [ir1] and check identifiers good use.
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<ir1::flow::Kind> {
                if ctx.ctx0.is_comp(&self.ident, false) {
                    // get called component id
                    let comp_id = ctx.ctx0.get_comp_id(&self.ident, false, ctx.errors)?;

                    let comp_inputs = ctx.get_comp_inputs(comp_id).clone();

                    // check inputs and comp_inputs have the same length
                    if self.inputs.len() != comp_inputs.len() {
                        bad!(ctx.errors, @ctx.loc =>
                            ErrorKind::arity_mismatch(self.inputs.len(), comp_inputs.len())
                        )
                    }

                    // transform inputs and map then to the identifiers of the component inputs
                    let inputs = {
                        let mut inputs = Vec::with_capacity(self.inputs.len());
                        for (input, id) in self.inputs.into_iter().zip(comp_inputs) {
                            inputs.push((id, input.into_ir1(ctx)?));
                        }
                        inputs
                    };

                    Ok(ir1::flow::Kind::comp_call(comp_id, inputs))
                } else {
                    // get called function id
                    let function_id = ctx.ctx0.get_function_id(&self.ident, false, ctx.errors)?;

                    let function_inputs = ctx.get_function_input(function_id).clone();

                    // check inputs and comp_inputs have the same length
                    if self.inputs.len() != function_inputs.len() {
                        bad!(ctx.errors, @ctx.loc =>
                            ErrorKind::arity_mismatch(self.inputs.len(), function_inputs.len())
                        )
                    }

                    // transform inputs and map then to the identifiers of the function inputs
                    let inputs = {
                        let mut inputs = Vec::with_capacity(self.inputs.len());
                        for (input, id) in self.inputs.into_iter().zip(function_inputs) {
                            inputs.push((id, input.into_ir1(ctx)?));
                        }
                        inputs
                    };

                    Ok(ir1::flow::Kind::fun_call(function_id, inputs))
                }
            }
        }

        impl<'a> Ir0IntoIr1<ir1::ctx::WithLoc<'a>> for FlowExpression {
            type Ir1 = flow::Expr;

            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression and check identifiers good use
            fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc<'a>) -> TRes<Self::Ir1> {
                let kind = match self {
                    FlowExpression::Ident(ident) => {
                        let id = ctx.ctx0.get_flow_id(&ident, false, ctx.errors)?;
                        flow::Kind::Ident { id }
                    }
                    FlowExpression::Call(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::Sample(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::Scan(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::Timeout(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::Throttle(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::OnChange(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::Persist(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::Merge(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::Time(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::Period(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::SampleOn(expr) => expr.into_ir1(ctx)?,
                    FlowExpression::ScanOn(expr) => expr.into_ir1(ctx)?,
                };
                Ok(flow::Expr::new(kind, ctx.loc))
            }
        }
    }
}

impl Ir0IntoIr1<ctx::Simple<'_>> for ir0::Function {
    type Ir1 = Function;

    // pre-condition: function and its inputs are already stored in symbol table
    // post-condition: construct [ir1] function and check identifiers good use
    fn into_ir1(self, ctx: &mut ctx::Simple) -> TRes<Self::Ir1> {
        let loc = self.loc();
        let id = ctx.ctx0.get_function_id(&self.ident, false, ctx.errors)?;

        // create local context with all idents
        ctx.local();
        ctx.restore_context(id);

        // insert function output type in symbol table
        let output_typing = self.output_type.into_ir1(&mut ctx.add_loc(loc))?;
        if !self.contract.clauses.is_empty() {
            let _ = ctx.ctx0.insert_function_result(
                output_typing.clone(),
                true,
                self.ident.loc(),
                ctx.errors,
            )?;
        }
        ctx.set_function_output_type(id, output_typing);

        let (mut statements, mut returned, mut logs) =
            (Vec::with_capacity(self.statements.len()), None, vec![]);
        for stmt in self.statements {
            match stmt {
                ir0::Stmt::Declaration(declaration) => {
                    statements.push(declaration.into_ir1(ctx)?);
                }
                ir0::Stmt::Return(ir0::stmt::Return { expression, .. }) => {
                    assert!(returned.is_none());
                    returned = Some(expression.into_ir1(&mut ctx.add_pat_loc(None, loc))?);
                }
                ir0::Stmt::Log(log_stmt) => {
                    let pat = log_stmt
                        .pattern
                        .into_ir1(&mut ctx.add_gen_loc(false, loc))?;
                    logs.extend(pat.identifiers());
                }
            }
        }
        let contract = self.contract.into_ir1(ctx)?;

        ctx.global();

        Ok(ir1::Function::new(
            id,
            contract,
            statements,
            logs,
            returned
                .ok_or_else(lerror!(@self.ident.loc() =>
                    "[internal] this function has not return expression"
                ))
                .dewrap(ctx.errors)?,
            loc,
        ))
    }
}

impl Ir0IntoIr1<ctx::Simple<'_>> for ir0::ExtFunDecl {
    type Ir1 = Function;

    // pre-condition: function and its inputs are already stored in symbol table
    // post-condition: construct [ir1] function and check identifiers good use
    fn into_ir1(self, ctx: &mut ctx::Simple) -> TRes<Self::Ir1> {
        let loc = self.loc();
        let id = ctx.ctx0.get_function_id(&self.ident, false, ctx.errors)?;

        // create local context with all idents
        ctx.local();
        ctx.restore_context(id);

        // insert function output type in symbol table
        let output_typing = self.output_typ().clone().into_ir1(&mut ctx.add_loc(loc))?;
        // let output_typing = self.output_typ.into_ir1(&mut ctx.add_loc(loc))?;
        ctx.set_function_output_type(id, output_typing);

        ctx.global();

        Ok(ir1::Function::new_ext(id, self.path, loc))
    }
}

impl Ir0IntoIr1<ctx::Simple<'_>> for ir0::ExtCompDecl {
    type Ir1 = ir1::Component;

    // pre-condition: component and its inputs are already stored in symbol table
    // post-condition: construct [ir1] component and check identifiers good use
    fn into_ir1(self, ctx: &mut ctx::Simple) -> TRes<Self::Ir1> {
        let loc = self.loc();
        let id = ctx.ctx0.get_comp_id(&self.ident, false, ctx.errors)?;

        Ok(ir1::Component::new_ext(id, self.path, loc))
    }
}

impl Ir0IntoIr1<ctx::Simple<'_>> for ir0::Component {
    type Ir1 = ir1::Component;

    // pre-condition: component and its idents are already stored in symbol table
    // post-condition: construct [ir1] component and check identifiers good use
    fn into_ir1(self, ctx: &mut ctx::Simple) -> TRes<Self::Ir1> {
        let loc = self.loc();
        let id = ctx.ctx0.get_comp_id(&self.ident, false, ctx.errors)?;

        // create local context with all idents
        ctx.local();
        ctx.restore_context(id);

        let mut inits = Vec::with_capacity(self.equations.len() / 2);
        let mut statements = Vec::with_capacity(self.equations.len());
        let mut logs = vec![];
        for react_eq in self.equations {
            match react_eq.into_ir1(ctx)? {
                StmtKind::Logs(ids) => logs.extend(ids),
                StmtKind::Init(init_stmt) => inits.push(init_stmt),
                StmtKind::Stmt(stmt, None) => statements.push(stmt),
                StmtKind::Stmt(stmt, Some(init_stmts)) => {
                    statements.push(stmt);
                    inits.extend(init_stmts)
                }
            }
        }
        statements.shrink_to_fit();

        let contract = self.contract.into_ir1(ctx)?;

        ctx.global();

        Ok(ir1::Component::new(
            id, inits, statements, contract, logs, loc,
        ))
    }
}

impl Ir0IntoIr1<ctx::Simple<'_>> for ir0::Contract {
    type Ir1 = Contract;

    fn into_ir1(self, ctx: &mut ctx::Simple) -> TRes<Self::Ir1> {
        use ir0::contract::ClauseKind::*;
        let (mut requires, mut ensures, mut invariant) = (
            Vec::with_capacity(self.clauses.len()),
            Vec::with_capacity(self.clauses.len()),
            Vec::with_capacity(self.clauses.len()),
        );
        for clause in self.clauses {
            match clause.kind {
                Requires(_) => requires.push(clause.term.into_ir1(ctx)?),
                Ensures(_) => ensures.push(clause.term.into_ir1(ctx)?),
                Invariant(_) => invariant.push(clause.term.into_ir1(ctx)?),
                Assert(a) => {
                    bad!(ctx.errors, @a.span =>
                        "`ir0::Contract::into_ir1` does not support assertions clauses yet"
                    )
                }
            }
        }
        requires.shrink_to_fit();
        ensures.shrink_to_fit();
        invariant.shrink_to_fit();

        Ok(ir1::Contract {
            requires,
            ensures,
            invariant,
        })
    }
}

impl<'a> Ir0IntoIr1<ctx::Simple<'a>> for ir0::contract::Term {
    type Ir1 = ir1::contract::Term;

    fn into_ir1(self, ctx: &mut ctx::Simple<'a>) -> TRes<Self::Ir1> {
        use ir0::contract::*;
        let loc = self.loc();
        match self {
            Term::Result(_) => {
                let id = ctx.ctx0.get_function_result_id(false, loc, ctx.errors)?;
                Ok(ir1::contract::Term::new(
                    ir1::contract::Kind::ident(id),
                    None,
                    loc,
                ))
            }
            Term::Implication(Implication { left, right, .. }) => {
                let left = left.into_ir1(ctx)?;
                let right = right.into_ir1(ctx)?;

                Ok(ir1::contract::Term::new(
                    ir1::contract::Kind::implication(left, right),
                    None,
                    loc,
                ))
            }
            Term::Application(app) if ctx.is_comp(&app.fun, false) => {
                let called_comp_id = ctx.ctx0.get_comp_id(&app.fun, false, ctx.errors)?;
                let comp_symbol = ctx
                    .get_symbol(called_comp_id)
                    .expect("internal error: there should be a symbol")
                    .clone();
                match comp_symbol.kind() {
                    SymbolKind::Component { inputs, .. } => {
                        // check inputs and comp_inputs have the same length
                        if inputs.len() != app.inputs.len() {
                            bad!(ctx.errors, @app.fun.loc() => ErrorKind::arity_mismatch(
                                inputs.len(), app.inputs.len()
                            ))
                        }

                        let inputs = res_vec!(
                            app.inputs.len(),
                            app.inputs
                                .into_iter()
                                .zip(inputs)
                                .map(|(input, id)| Ok((*id, input.into_ir1(ctx)?))),
                        );

                        Ok(ir1::contract::Term::new(
                            ir1::contract::Kind::call(called_comp_id, inputs),
                            None,
                            loc,
                        ))
                    }
                    _ => {
                        ctx.errors.push(error!( @ comp_symbol.name().span() =>
                            "fatal: symbol kind associated to component `{}` is not component-like",
                            comp_symbol.name(),
                        ));
                        Err(ErrorDetected)
                    }
                }
            }
            Term::Application(app) => {
                let fun_id = ctx.ctx0.get_function_id(&app.fun, false, ctx.errors)?;
                let inputs = res_vec!(
                    app.inputs.len(),
                    app.inputs.into_iter().map(|term| term.into_ir1(ctx))
                );

                Ok(ir1::contract::Term::new(
                    ir1::contract::Kind::app(fun_id, inputs),
                    None,
                    loc,
                ))
            }
            Term::Enumeration(Enumeration {
                enum_name,
                elem_name,
            }) => {
                let enum_id = ctx.ctx0.get_enum_id(&enum_name, false, loc, ctx.errors)?;
                let element_id = ctx
                    .ctx0
                    .get_enum_elem_id(&elem_name, &enum_name, false, loc, ctx.errors)?;
                Ok(ir1::contract::Term::new(
                    ir1::contract::Kind::enumeration(enum_id, element_id),
                    None,
                    loc,
                ))
            }
            Term::Paren(term) => Ok(ir1::contract::Term::new(
                ir1::contract::Kind::paren(term.into_ir1(ctx)?),
                None,
                loc,
            )),
            Term::Unary(Unary { op, term, .. }) => Ok(ir1::contract::Term::new(
                ir1::contract::Kind::unary(op, term.into_ir1(ctx)?),
                None,
                loc,
            )),
            Term::Binary(Binary {
                op, left, right, ..
            }) => Ok(ir1::contract::Term::new(
                ir1::contract::Kind::binary(op, left.into_ir1(ctx)?, right.into_ir1(ctx)?),
                None,
                loc,
            )),
            Term::Constant(constant) => Ok(ir1::contract::Term::new(
                ir1::contract::Kind::constant(constant),
                None,
                loc,
            )),
            Term::Identifier(ident) => {
                let id = ctx.ctx0.get_ident(&ident, false, false, ctx.errors)?;
                if let Some(value) = ctx.ctx0.try_get_const(id) {
                    let term: ir0::contract::Term = value.clone().try_into().dewrap(ctx.errors)?;
                    term.into_ir1(ctx)
                } else {
                    Ok(ir1::contract::Term::new(
                        ir1::contract::Kind::ident(id),
                        None,
                        loc,
                    ))
                }
            }
            Term::Last(ident) => {
                let init_id = ctx.ctx0.get_init_id(&ident, false, ctx.errors)?;
                let ident_id = ctx.ctx0.get_ident(&ident, false, false, ctx.errors)?;
                Ok(ir1::contract::Term::new(
                    ir1::contract::Kind::last(init_id, ident_id),
                    None,
                    loc,
                ))
            }
            Term::ForAll(ForAll {
                ident, ty, term, ..
            }) => {
                let ty = ty.into_ir1(&mut ctx.add_loc(loc))?;
                ctx.local();
                let id = ctx
                    .ctx0
                    .insert_local_ident(ident.clone(), Some(ty), true, ctx.errors)?;
                let term = term.into_ir1(ctx)?;
                ctx.global();
                Ok(ir1::contract::Term::new(
                    ir1::contract::Kind::forall(id, term),
                    None,
                    loc,
                ))
            }
            Term::EventImplication(EventImplication {
                pattern,
                event,
                term,
                ..
            }) => {
                // get the event identifier
                let event_id = ctx.ctx0.get_identifier_id(&event, false, ctx.errors)?;
                ctx.local();
                // set pattern ident in local context
                let pattern_id =
                    ctx.ctx0
                        .insert_local_ident(pattern.clone(), None, true, ctx.errors)?;
                // transform term into [ir1]
                let right = term.into_ir1(ctx)?;
                ctx.global();
                // construct right side of implication: `PresentEvent(pat) == event`
                let left = ir1::contract::Term::new(
                    ir1::contract::Kind::binary(
                        BOp::Eq,
                        ir1::contract::Term::new(
                            ir1::contract::Kind::present(event_id, pattern_id),
                            None,
                            loc,
                        ),
                        ir1::contract::Term::new(ir1::contract::Kind::ident(event_id), None, loc),
                    ),
                    None,
                    loc,
                );
                // construct result term
                // - `when pat = e? => t`
                // becomes
                // - `forall pat, PresentEvent(pat) == event => t`
                let term = ir1::contract::Term::new(
                    ir1::contract::Kind::forall(
                        pattern_id,
                        ir1::contract::Term::new(
                            ir1::contract::Kind::implication(left, right),
                            None,
                            loc,
                        ),
                    ),
                    None,
                    loc,
                );
                Ok(term)
            }
        }
    }
}

impl Ir0IntoIr1<ctx::Simple<'_>> for ir0::Eq {
    type Ir1 = ir1::stream::Stmt;

    /// Pre-condition: equation's ident is already stored in symbol table.
    ///
    /// Post-condition: construct [ir1] equation and check identifiers good use.
    fn into_ir1(self, ctx: &mut ctx::Simple) -> TRes<Self::Ir1> {
        use ir0::{
            equation::{Arm, Eq, Instantiation, MatchEq},
            stmt::LetDecl,
        };
        let loc = self.loc();

        // get idents defined by the equation
        let mut defined_idents = HashMap::new();
        self.get_idents(&mut defined_idents);

        match self {
            Eq::LocalDef(LetDecl {
                expr,
                typed_pattern: pattern,
                ..
            })
            | Eq::OutputDef(Instantiation { expr, pattern, .. }) => {
                let expr = expr.into_ir1(&mut ctx.add_pat_loc(Some(&pattern), loc))?;
                let pattern = pattern.into_ir1(&mut ctx.add_gen_loc(false, loc))?;
                Ok(ir1::Stmt { pattern, expr, loc })
            }
            Eq::MatchEq(MatchEq { expr, arms, .. }) => {
                // create the receiving pattern for the equation
                let pattern = {
                    let mut elements = res_vec!(
                        defined_idents.len(),
                        defined_idents
                            .values()
                            .map(|pat| pat.clone().into_ir1(&mut ctx.add_gen_loc(true, loc))),
                    );
                    if elements.len() == 1 {
                        elements.pop().unwrap()
                    } else {
                        stmt::Pattern::new(loc, stmt::Kind::tuple(elements))
                    }
                };

                // for each arm, construct pattern guard and statements
                let mut arm_exprs = Vec::with_capacity(arms.len());
                for Arm {
                    pattern,
                    guard,
                    equations,
                    ..
                } in arms.into_iter()
                {
                    // transform pattern guard and equations into [ir1]
                    let (idents, pattern, guard, statements) = {
                        ctx.local();

                        // set local context: pattern idents + equations' idents
                        pattern.store(ctx)?;
                        let mut idents = HashMap::new();
                        equations
                            .iter()
                            .map(|equation| {
                                // store equations' idents in the local context
                                equation.store_idents(true, &mut idents, ctx)
                            })
                            .collect_res()?;

                        // transform pattern guard and equations into [ir1] with local
                        // context
                        let pattern = pattern.into_ir1(&mut ctx.add_loc(loc))?;
                        let guard = guard
                            .map(|(_, expression)| {
                                expression.into_ir1(&mut ctx.add_pat_loc(None, loc))
                            })
                            .transpose()?;
                        let statements = res_vec!(
                            equations.len(),
                            equations.into_iter().map(|equation| equation.into_ir1(ctx)),
                        );

                        ctx.global();

                        (idents, pattern, guard, statements)
                    };

                    // create the tuple expression
                    let expression = {
                        // check defined idents are all the same
                        if defined_idents.len() != idents.len() {
                            bad!(ctx.errors, @loc => ErrorKind::incompatible_match(
                                defined_idents.len(), idents.len(),
                            ))
                        }
                        let mut elements = {
                            let mut elements = Vec::with_capacity(defined_idents.len());
                            for ident_name in defined_idents.keys() {
                                if let Some(id) = idents.get(ident_name) {
                                    elements.push(ir1::stream::Expr::new(
                                        ident_name.loc(),
                                        ir1::stream::Kind::expr(ir1::expr::Kind::ident(*id)),
                                    ));
                                } else {
                                    bad!(ctx.errors, @loc =>
                                        ErrorKind::missing_match_stmt(ident_name.to_string())
                                    )
                                }
                            }
                            elements
                        };

                        // create the tuple expression
                        if elements.len() == 1 {
                            elements.pop().unwrap()
                        } else {
                            stream::Expr::new(
                                pattern.loc(),
                                stream::Kind::expr(ir1::expr::Kind::tuple(elements)),
                            )
                        }
                    };

                    arm_exprs.push((pattern, guard, statements, expression));
                }
                // construct the match expression
                let expr = stream::Expr::new(
                    loc,
                    stream::Kind::expr(ir1::expr::Kind::match_expr(
                        expr.into_ir1(&mut ctx.add_pat_loc(None, loc))?,
                        arm_exprs,
                    )),
                );

                Ok(ir1::Stmt { pattern, expr, loc })
            }
        }
    }
}

/// Used as an output type for the implementation of [`Ir0IntoIr1`] for [`ir0::ReactEq`].
pub enum StmtKind {
    Logs(Vec<usize>),
    Init(stream::InitStmt),
    Stmt(stream::Stmt, Option<Vec<stream::InitStmt>>),
}

impl Ir0IntoIr1<ctx::Simple<'_>> for ir0::ReactEq {
    /// Reactive equations like `init` or `when` contains initializations (one or many) that are
    /// also part of their [ir1] representation, along the (optional) [ir1::stmt::Stmt].
    type Ir1 = StmtKind;

    /// Pre-condition: equation's ident is already stored in symbol table.
    ///
    /// Post-condition: construct [ir1] equation and check identifiers good use.
    fn into_ir1(self, ctx: &mut ctx::Simple) -> TRes<Self::Ir1> {
        use ir0::{
            equation::{Arm, EventArmWhen, Instantiation, WhenEq},
            stmt::LetDecl,
            ReactEq,
        };
        let loc = self.loc();

        // get idents defined by the equation
        let mut defined_idents = HashMap::new();
        self.get_idents(&mut defined_idents);

        match self {
            ReactEq::LocalDef(LetDecl {
                expr,
                typed_pattern: pattern,
                ..
            })
            | ReactEq::OutputDef(Instantiation { expr, pattern, .. }) => {
                let (init_stmts, expr) =
                    expr.into_ir1(&mut ctx.add_pat_loc(Some(&pattern), loc))?;
                let pattern = pattern.into_ir1(&mut ctx.add_gen_loc(false, loc))?;
                Ok(StmtKind::Stmt(
                    stream::Stmt { pattern, expr, loc },
                    Some(init_stmts),
                ))
            }
            ReactEq::MatchEq(ir0::equation::MatchEq { expr, arms, .. }) => {
                // create the receiving pattern for the equation
                let pattern = {
                    let mut elements = res_vec!(
                        defined_idents.len(),
                        defined_idents
                            .values()
                            .map(|pat| pat.clone().into_ir1(&mut ctx.add_gen_loc(true, loc))),
                    );
                    if elements.len() == 1 {
                        elements.pop().unwrap()
                    } else {
                        stmt::Pattern::new(loc, stmt::Kind::tuple(elements))
                    }
                };

                // for each arm, construct pattern guard and statements
                let mut arm_exprs = Vec::with_capacity(arms.len());
                for Arm {
                    pattern,
                    guard,
                    equations,
                    ..
                } in arms.into_iter()
                {
                    // transform pattern guard and equations into [ir1]
                    let (idents, pattern, guard, statements) = {
                        ctx.local();

                        // set local context: pattern idents + equations' idents
                        pattern.store(ctx)?;
                        let mut idents = HashMap::new();
                        equations
                            .iter()
                            .map(|equation| {
                                // store equations' idents in the local context
                                equation.store_idents(true, &mut idents, ctx)
                            })
                            .collect_res()?;

                        // transform pattern guard and equations into [ir1] with local
                        // context
                        let pattern = pattern.into_ir1(&mut ctx.add_loc(loc))?;
                        let guard = guard
                            .map(|(_, expression)| {
                                expression.into_ir1(&mut ctx.add_pat_loc(None, loc))
                            })
                            .transpose()?;
                        let statements = res_vec!(
                            equations.len(),
                            equations.into_iter().map(|equation| equation.into_ir1(ctx)),
                        );

                        ctx.global();

                        (idents, pattern, guard, statements)
                    };

                    // create the tuple expression
                    let expression = {
                        // check defined idents are all the same
                        if defined_idents.len() != idents.len() {
                            bad!(ctx.errors, @loc => ErrorKind::incompatible_match(
                                defined_idents.len(), idents.len(),
                            ))
                        }
                        let mut elements = {
                            let mut elements = Vec::with_capacity(defined_idents.len());
                            for ident_name in defined_idents.keys() {
                                if let Some(id) = idents.get(ident_name) {
                                    elements.push(ir1::stream::Expr::new(
                                        ident_name.loc(),
                                        ir1::stream::Kind::expr(ir1::expr::Kind::ident(*id)),
                                    ));
                                } else {
                                    bad!(ctx.errors, @loc =>
                                        ErrorKind::missing_match_stmt(ident_name.to_string())
                                    )
                                }
                            }
                            elements
                        };

                        // create the tuple expression
                        if elements.len() == 1 {
                            elements.pop().unwrap()
                        } else {
                            stream::Expr::new(
                                pattern.loc(),
                                stream::Kind::expr(ir1::expr::Kind::tuple(elements)),
                            )
                        }
                    };

                    arm_exprs.push((pattern, guard, statements, expression));
                }
                // construct the match expression
                let expr = stream::Expr::new(
                    loc,
                    stream::Kind::expr(ir1::expr::Kind::match_expr(
                        expr.into_ir1(&mut ctx.add_pat_loc(None, loc))?,
                        arm_exprs,
                    )),
                );

                Ok(StmtKind::Stmt(stream::Stmt { pattern, expr, loc }, None))
            }
            ReactEq::WhenEq(WhenEq { init, arms, .. }) => {
                // create the pattern defined by the equation
                let def_eq_pat = {
                    let mut elements: Vec<_> = defined_idents.into_values().collect();
                    if elements.len() == 1 {
                        elements.pop().unwrap()
                    } else {
                        ir0::stmt::Pattern::tuple(ir0::stmt::Tuple::new(loc, elements))
                    }
                };

                // idents initial values
                let (init_stmts, signal_idents) =
                    init.map_or(Ok((vec![], vec![])), |init_arm| {
                        let mut init_stmts = Vec::with_capacity(init_arm.equations.len());
                        let mut signal_idents = Vec::with_capacity(init_arm.equations.len());
                        for eq in init_arm.equations {
                            eq.expr.check_is_constant(ctx.ctx0, ctx.errors)?;
                            let init_ids = eq
                                .pattern
                                .get_idents()
                                .into_iter()
                                .map(|(ident, _)| {
                                    let id =
                                        ctx.ctx0.get_identifier_id(&ident, false, ctx.errors)?;
                                    let init_id =
                                        ctx.ctx0.get_init_id(&ident, false, ctx.errors)?;
                                    signal_idents.push((ident, id));
                                    Ok((id, init_id))
                                })
                                .collect::<TRes<HashMap<_, _>>>()?;
                            let ir1_pat = eq.pattern.into_ir1(&mut ctx.add_gen_loc(false, loc))?;
                            let ir1_expr = eq.expr.into_ir1(&mut ctx.add_pat_loc(None, loc))?;
                            init_stmts.push(stream::InitStmt {
                                pattern: ir1_pat,
                                expr: ir1_expr,
                                init_ids,
                                loc,
                            })
                        }
                        Ok((init_stmts, signal_idents))
                    })?;

                // for each arm construct [ir1] pattern, guard and statements
                let mut new_arms = Vec::with_capacity(arms.len());
                for EventArmWhen {
                    pattern: event_pattern,
                    guard,
                    equations,
                    ..
                } in arms.into_iter()
                {
                    // transform event_pattern guard and equations into [ir1]
                    ctx.local();

                    // set local context: equations' idents + signal_ids
                    let mut idents = HashMap::new();
                    equations
                        .iter()
                        .map(|equation| {
                            // store equations' idents in the local context
                            equation.store_idents(true, &mut idents, ctx)
                        })
                        .collect_res()?;
                    signal_idents
                        .iter()
                        .filter(|(ident, _)| !idents.contains_key(ident))
                        .try_for_each(|(ident, id)| {
                            let ident = ident.clone();
                            // should be already typed
                            let typ = ctx.get_typ(*id).clone();
                            let _id =
                                ctx.ctx0
                                    .insert_local_ident(ident, Some(typ), true, ctx.errors)?;
                            Ok(())
                        })?;

                    let pat_loc = event_pattern.loc();
                    // transform event_pattern into [ir1] + set local context
                    let event_pattern = event_pattern.into_ir1(&mut ctx.add_loc(pat_loc))?;
                    // transform AST guard into [ir1]
                    let ir1_guard = guard
                        .map(|(_, expr)| expr.into_ir1(&mut ctx.add_pat_loc(None, pat_loc)))
                        .transpose()?;

                    // transform equations into [ir1] with local context
                    let statements = res_vec!(
                        equations.len(),
                        equations.into_iter().map(|equation| equation.into_ir1(ctx)),
                    );

                    ctx.global();

                    new_arms.push((event_pattern, ir1_guard, statements))
                }

                // transform stmt pattern into [ir1]
                let pattern = def_eq_pat.into_ir1(&mut ctx.add_gen_loc(true, loc))?;

                // signals memory values
                let init_ids = {
                    let mut init_ids = HashMap::with_capacity(init_stmts.len());
                    for init_stmt in &init_stmts {
                        init_ids.extend(init_stmt.init_ids.clone());
                    }
                    init_ids
                };

                // construct the when expression
                let when_expr =
                    stream::Expr::new(loc, stream::Kind::when(pattern.clone(), init_ids, new_arms));

                Ok(StmtKind::Stmt(
                    stream::Stmt {
                        pattern,
                        expr: when_expr,
                        loc,
                    },
                    Some(init_stmts),
                ))
            }
            ReactEq::Init(init_ident) => {
                init_ident.expr.check_is_constant(ctx.ctx0, ctx.errors)?;
                let init_ids = init_ident
                    .pattern
                    .get_idents()
                    .into_iter()
                    .map(|(ident, _)| {
                        Ok((
                            ctx.ctx0.get_identifier_id(&ident, false, ctx.errors)?,
                            ctx.ctx0.get_init_id(&ident, false, ctx.errors)?,
                        ))
                    })
                    .collect::<TRes<HashMap<_, _>>>()?;
                let ir1_pat = init_ident
                    .pattern
                    .into_ir1(&mut ctx.add_gen_loc(false, loc))?;
                let ir1_expr = init_ident.expr.into_ir1(&mut ctx.add_pat_loc(None, loc))?;
                let init_stmt = stream::InitStmt {
                    pattern: ir1_pat,
                    expr: ir1_expr,
                    init_ids,
                    loc,
                };
                Ok(StmtKind::Init(init_stmt))
            }
            ReactEq::Log(log_stmt) => {
                let pat = log_stmt
                    .pattern
                    .into_ir1(&mut ctx.add_gen_loc(false, loc))?;
                Ok(StmtKind::Logs(pat.identifiers()))
            }
        }
    }
}

impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for ir0::expr::UnOp<E>
where
    E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
{
    type Ir1 = expr::Kind<E::Ir1>;

    /// Transforms AST into [ir1] and check identifiers good use.
    fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
        // pre-condition: identifiers are stored in symbol table
        // post-condition: construct [ir1] expression kind and check identifiers good use
        Ok(expr::Kind::unop(self.op, self.expr.into_ir1(ctx)?))
    }
}

impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for ir0::expr::BinOp<E>
where
    E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
{
    type Ir1 = expr::Kind<E::Ir1>;

    /// Transforms AST into [ir1] and check identifiers good use.
    fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
        // pre-condition: identifiers are stored in symbol table
        // post-condition: construct [ir1] expression kind and check identifiers good use
        Ok(expr::Kind::binop(
            self.op,
            self.lft.into_ir1(ctx)?,
            self.rgt.into_ir1(ctx)?,
        ))
    }
}

impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for ir0::expr::IfThenElse<E>
where
    E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
{
    type Ir1 = expr::Kind<E::Ir1>;

    /// Transforms AST into [ir1] and check identifiers good use.
    fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
        // pre-condition: identifiers are stored in symbol table
        // post-condition: construct [ir1] expression kind and check identifiers good use
        Ok(expr::Kind::if_then_else(
            self.cnd.into_ir1(ctx)?,
            self.thn.into_ir1(ctx)?,
            self.els.into_ir1(ctx)?,
        ))
    }
}

impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for ir0::expr::Application<E>
where
    E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
{
    type Ir1 = expr::Kind<E::Ir1>;

    /// Transforms AST into [ir1] and check identifiers good use.
    fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
        // pre-condition: identifiers are stored in symbol table
        // post-condition: construct [ir1] expression kind and check identifiers good use
        Ok(expr::Kind::app(
            self.fun.into_ir1(ctx)?,
            res_vec!(
                self.inputs.len(),
                self.inputs.into_iter().map(|input| input.into_ir1(ctx)),
            ),
        ))
    }
}

impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for ir0::expr::Structure<E>
where
    E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
{
    type Ir1 = expr::Kind<E::Ir1>;

    /// Transforms AST into [ir1] and check identifiers good use.
    fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
        // pre-condition: identifiers are stored in symbol table
        // post-condition: construct [ir1] expression kind and check identifiers good use
        let id = ctx
            .ctx0
            .get_struct_id(&self.name, false, ctx.loc, ctx.errors)?;
        let mut field_ids = ctx
            .get_struct_fields(id)
            .clone()
            .into_iter()
            .map(|id| (ctx.get_name(id).clone(), id))
            .collect::<HashMap<_, _>>();

        let fields = res_vec!(
            self.fields.len(),
            self.fields.into_iter().map(|(field_name, expression)| {
                let id = field_ids.remove(&field_name).map_or_else(
                    || {
                        bad!(ctx.errors, @ctx.loc =>
                            ErrorKind::unknown_field(self.name.to_string(), field_name.to_string())
                        )
                    },
                    Ok,
                )?;
                let expression = expression.into_ir1(ctx)?;
                Ok((id, expression))
            }),
        );

        // fail on missing fields
        if let Some(field_name) = field_ids.keys().next() {
            bad!(ctx.errors, @self.loc =>
                ErrorKind::missing_field(self.name.to_string(), field_name.to_string())
                => | @field_name.loc() => "field declared here"
            )
        }

        Ok(expr::Kind::Structure { id, fields })
    }
}

mod simple_expr_impl {
    prelude! {
        ir0::expr::*,
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for Enumeration<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>>
        where
            E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
        {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            let enum_id = ctx
                .ctx0
                .get_enum_id(&self.enum_name, false, ctx.loc, ctx.errors)?;
            let elem_id = ctx.ctx0.get_enum_elem_id(
                &self.elem_name,
                &self.enum_name,
                false,
                ctx.loc,
                ctx.errors,
            )?;
            Ok(expr::Kind::Enumeration { enum_id, elem_id })
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for Array<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            Ok(expr::Kind::Array {
                elements: res_vec!(
                    self.elements.len(),
                    self.elements
                        .into_iter()
                        .map(|expression| expression.into_ir1(ctx)),
                ),
            })
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for Tuple<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            Ok(expr::Kind::tuple(res_vec!(
                self.elements.len(),
                self.elements
                    .into_iter()
                    .map(|expression| expression.into_ir1(ctx)),
            )))
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for MatchExpr<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            Ok(expr::Kind::match_expr(
                self.expr.into_ir1(ctx)?,
                res_vec!(
                    self.arms.len(),
                    self.arms.into_iter().map(|arm| {
                        ctx.local();
                        arm.pattern.store(&mut ctx.rm_pat_loc())?;
                        let pattern = arm.pattern.into_ir1(&mut ctx.rm_pat())?;
                        let guard = arm.guard.map(|expr| expr.into_ir1(ctx)).transpose()?;
                        let expr = arm.expr.into_ir1(ctx)?;
                        ctx.global();
                        Ok((pattern, guard, vec![], expr))
                    }),
                ),
            ))
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for FieldAccess<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            Ok(expr::Kind::field_access(
                self.expr.into_ir1(ctx)?,
                self.field,
            ))
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for TupleElementAccess<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            Ok(expr::Kind::tuple_access(
                self.expr.into_ir1(ctx)?,
                self.element_number,
            ))
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for ArrayAccess<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            Ok(expr::Kind::array_access(
                self.expr.into_ir1(ctx)?,
                self.index,
            ))
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for Map<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            Ok(expr::Kind::map(
                self.expr.into_ir1(ctx)?,
                self.fun.into_ir1(ctx)?,
            ))
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for Fold<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            Ok(expr::Kind::fold(
                self.array.into_ir1(ctx)?,
                self.init.into_ir1(ctx)?,
                self.fun.into_ir1(ctx)?,
            ))
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for Sort<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            Ok(expr::Kind::sort(
                self.expr.into_ir1(ctx)?,
                self.fun.into_ir1(ctx)?,
            ))
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for Zip<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            Ok(expr::Kind::zip(res_vec!(
                self.arrays.len(),
                self.arrays.into_iter().map(|array| array.into_ir1(ctx)),
            )))
        }
    }

    impl<'a, E> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for Lambda<E>
    where
        E: Ir0IntoIr1<ir1::ctx::PatLoc<'a>>,
    {
        type Ir1 = expr::Kind<E::Ir1>;

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<expr::Kind<E::Ir1>> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            ctx.local();
            let inputs = res_vec!(
                self.inputs.len(),
                self.inputs.into_iter().map(|(input_name, typing)| {
                    let typing = typing.into_ir1(&mut ctx.rm_pat())?;
                    ctx.ctx0
                        .insert_local_ident(input_name, Some(typing), true, ctx.errors)
                }),
            );
            let expr = self.expr.into_ir1(ctx)?;
            ctx.global();

            Ok(expr::Kind::lambda(inputs, expr))
        }
    }

    impl<'a> Ir0IntoIr1<ir1::ctx::PatLoc<'a>> for ir0::Expr {
        type Ir1 = ir1::Expr;

        // pre-condition: identifiers are stored in symbol table
        // post-condition: construct [ir1] expression and check identifiers good use
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc<'a>) -> TRes<Self::Ir1> {
            use ir0::Expr::*;
            let kind = match self {
                Constant(constant) => ir1::expr::Kind::Constant { constant },
                Identifier(id) => {
                    let id = ctx.ctx0.get_ident(&id, false, true, ctx.errors)?;
                    if let Some(value) = ctx.ctx0.try_get_const(id) {
                        return value.clone().into_ir1(ctx);
                    } else {
                        ir1::expr::Kind::ident(id)
                    }
                }
                UnOp(e) => e.into_ir1(ctx)?,
                BinOp(e) => e.into_ir1(ctx)?,
                IfThenElse(e) => e.into_ir1(ctx)?,
                Application(e) => e.into_ir1(ctx)?,
                Lambda(e) => e.into_ir1(ctx)?,
                Structure(e) => e.into_ir1(ctx)?,
                Tuple(e) => e.into_ir1(ctx)?,
                Enumeration(e) => e.into_ir1(ctx)?,
                Array(e) => e.into_ir1(ctx)?,
                MatchExpr(e) => e.into_ir1(ctx)?,
                FieldAccess(e) => e.into_ir1(ctx)?,
                TupleElementAccess(e) => e.into_ir1(ctx)?,
                ArrayAccess(e) => e.into_ir1(ctx)?,
                Map(e) => e.into_ir1(ctx)?,
                Fold(e) => e.into_ir1(ctx)?,
                Sort(e) => e.into_ir1(ctx)?,
                Zip(e) => e.into_ir1(ctx)?,
            };
            Ok(ir1::Expr::new(kind, ctx.loc))
        }
    }
}

mod expr_pattern_impl {
    prelude! {
        ir0::expr::{PatEnumeration, PatStructure, PatTuple},
    }

    impl Ir0IntoIr1<ir1::ctx::WithLoc<'_>> for PatStructure {
        type Ir1 = ir1::pattern::Kind;

        fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc) -> TRes<ir1::pattern::Kind> {
            let loc = self.loc();
            let id = ctx
                .ctx0
                .get_struct_id(&self.name, false, ctx.loc, ctx.errors)?;
            let mut field_ids = ctx
                .get_struct_fields(id)
                .iter()
                .map(|id| (ctx.get_name(*id).clone(), *id))
                .collect::<HashMap<_, _>>();

            let fields = res_vec!(
                self.fields.len(),
                self.fields
                    .into_iter()
                    .map(|(field_name, optional_pattern)| {
                        let id = field_ids.remove(&field_name).map_or_else(
                            || {
                                bad!(ctx.errors, @ctx.loc => ErrorKind::unknown_field(
                                    self.name.to_string(), field_name.to_string(),
                                ))
                            },
                            Ok,
                        )?;
                        let pattern = optional_pattern
                            .map(|pattern| pattern.into_ir1(ctx))
                            .transpose()?;
                        Ok((id, pattern))
                    }),
            );

            if self.rest.is_none() {
                // check if there are no missing fields
                if let Some(field_name) = field_ids.keys().next() {
                    bad!(ctx.errors, @loc =>
                        ErrorKind::missing_field(self.name.to_string(), field_name.to_string())
                        => | @field_name.loc() => "field declared here"
                    )
                }
            }

            Ok(ir1::pattern::Kind::Structure { id, fields })
        }
    }

    impl Ir0IntoIr1<ir1::ctx::WithLoc<'_>> for PatEnumeration {
        type Ir1 = ir1::pattern::Kind;

        fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc) -> TRes<ir1::pattern::Kind> {
            let enum_id = ctx
                .ctx0
                .get_enum_id(&self.enum_name, false, ctx.loc, ctx.errors)?;
            let elem_id = ctx.ctx0.get_enum_elem_id(
                &self.elem_name,
                &self.enum_name,
                false,
                ctx.loc,
                ctx.errors,
            )?;
            Ok(ir1::pattern::Kind::Enumeration { enum_id, elem_id })
        }
    }

    impl Ir0IntoIr1<ir1::ctx::WithLoc<'_>> for PatTuple {
        type Ir1 = ir1::pattern::Kind;

        fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc) -> TRes<ir1::pattern::Kind> {
            Ok(ir1::pattern::Kind::tuple(res_vec!(
                self.elements.len(),
                self.elements
                    .into_iter()
                    .map(|pattern| pattern.into_ir1(ctx)),
            )))
        }
    }

    impl Ir0IntoIr1<ir1::ctx::WithLoc<'_>> for ir0::expr::Pattern {
        type Ir1 = ir1::Pattern;

        fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc) -> TRes<Self::Ir1> {
            use ir0::expr::Pattern::*;
            let kind = match self {
                Constant(constant) => ir1::pattern::Kind::Constant { constant },
                Identifier(name) => {
                    let id = ctx.ctx0.get_identifier_id(&name, false, ctx.errors)?;
                    ir1::pattern::Kind::Identifier { id }
                }
                Structure(pat) => pat.into_ir1(ctx)?,
                Enumeration(pat) => pat.into_ir1(ctx)?,
                Tuple(pat) => pat.into_ir1(ctx)?,
                // None => ir1::pattern::Kind::None,
                Default(loc) => ir1::pattern::Kind::Default(loc),
            };

            Ok(ir1::Pattern::new(ctx.loc, kind))
        }
    }
}

mod event_pattern_impl {
    prelude! {
        ir0::equation::{EventPattern, LetEventPattern, TupleEventPattern},
    }

    impl Ir0IntoIr1<ir1::ctx::WithLoc<'_>> for LetEventPattern {
        type Ir1 = ir1::event_pattern::Kind;

        fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc) -> TRes<Self::Ir1> {
            let event_id = ctx.ctx0.get_identifier_id(&self.event, false, ctx.errors)?;
            // put pattern idents in local context
            self.pattern.store(&mut ctx.rm_loc())?;
            let pattern = self.pattern.into_ir1(ctx)?;
            Ok(ir1::event_pattern::Kind::detect(pattern, event_id))
        }
    }

    impl Ir0IntoIr1<ir1::ctx::WithLoc<'_>> for TupleEventPattern {
        type Ir1 = ir1::event_pattern::Kind;

        fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc) -> TRes<Self::Ir1> {
            Ok(ir1::event_pattern::Kind::tuple(res_vec!(
                self.patterns.len(),
                self.patterns
                    .into_iter()
                    .map(|pattern| pattern.into_ir1(ctx)),
            )))
        }
    }

    impl Ir0IntoIr1<ir1::ctx::WithLoc<'_>> for EventPattern {
        type Ir1 = ir1::EventPattern;

        fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc) -> TRes<Self::Ir1> {
            use ir0::equation::EventPattern::*;
            let kind = match self {
                Tuple(pat) => pat.into_ir1(ctx)?,
                Let(pat) => pat.into_ir1(ctx)?,
                RisingEdge(expr) => {
                    ir1::event_pattern::Kind::rising_edge(expr.into_ir1(&mut ctx.add_pat(None))?)
                }
            };

            Ok(ir1::EventPattern {
                kind,
                typing: None,
                loc: ctx.loc,
            })
        }
    }
}

mod stmt_pattern_impl {
    prelude! {
        ir0::stmt::{Typed, Tuple}
    }

    impl Ir0IntoIr1<ir1::ctx::GenLoc<'_>> for Typed {
        type Ir1 = ir1::stmt::Kind;

        fn into_ir1(self, ctx: &mut ir1::ctx::GenLoc) -> TRes<ir1::stmt::Kind> {
            let id = ctx.ctx0.get_identifier_id(&self.ident, false, ctx.errors)?;
            if ctx.codegen {
                ctx.ctx0.set_codegen(id)
            }
            let typ = self.typ.into_ir1(&mut ctx.rm_loc())?;
            Ok(ir1::stmt::Kind::Typed { id, typ })
        }
    }

    impl Ir0IntoIr1<ir1::ctx::GenLoc<'_>> for Tuple {
        type Ir1 = ir1::stmt::Kind;

        fn into_ir1(self, ctx: &mut ir1::ctx::GenLoc) -> TRes<ir1::stmt::Kind> {
            Ok(ir1::stmt::Kind::tuple(res_vec!(
                self.elements.len(),
                self.elements
                    .into_iter()
                    .map(|pattern| pattern.into_ir1(ctx)),
            )))
        }
    }

    impl Ir0IntoIr1<ir1::ctx::GenLoc<'_>> for ir0::stmt::Pattern {
        type Ir1 = ir1::stmt::Pattern;

        fn into_ir1(self, ctx: &mut ir1::ctx::GenLoc) -> TRes<Self::Ir1> {
            use ir0::stmt::Pattern::*;
            let kind = match self {
                Identifier(ident) => {
                    let id = ctx.ctx0.get_identifier_id(&ident, false, ctx.errors)?;
                    if ctx.codegen {
                        ctx.ctx0.set_codegen(id)
                    }
                    ir1::stmt::Kind::Identifier { id }
                }
                Typed(pattern) => pattern.into_ir1(ctx)?,
                Tuple(pattern) => pattern.into_ir1(ctx)?,
            };

            Ok(ir1::stmt::Pattern {
                kind,
                typ: None,
                loc: ctx.loc,
            })
        }
    }
}

impl Ir0IntoIr1<ir1::ctx::Simple<'_>> for ir0::stmt::LetDecl<ir0::Expr> {
    type Ir1 = ir1::Stmt<ir1::Expr>;

    // pre-condition: NOTHING is in symbol table
    // post-condition: construct [ir1] statement and check identifiers good use
    fn into_ir1(self, ctx: &mut ir1::ctx::Simple) -> TRes<Self::Ir1> {
        let loc = self.loc();
        // stmts should be ordered in functions
        // then patterns are stored in order
        self.typed_pattern.store(ctx)?;
        let expr = self
            .expr
            .into_ir1(&mut ctx.add_pat_loc(Some(&self.typed_pattern), loc))?;
        let pattern = self
            .typed_pattern
            .into_ir1(&mut ctx.add_gen_loc(false, loc))?;

        Ok(ir1::Stmt { pattern, expr, loc })
    }
}

mod stream_impl {
    prelude! { ir0::{ symbol::SymbolKind, stream } }

    impl Ir0IntoIr1<ir1::ctx::PatLoc<'_>> for stream::WhenExpr {
        /// Reactive expressions `when` can contain initializations that are
        /// also part of their [ir1] representation, along the [ir1::stream::Kind].
        type Ir1 = (Vec<ir1::stream::InitStmt>, ir1::stream::Kind);

        /// Transforms AST into [ir1] and check identifiers good use.
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc) -> TRes<Self::Ir1> {
            // pre-condition: identifiers are stored in symbol table
            // post-condition: construct [ir1] expression kind and check identifiers good use
            let stream::WhenExpr { init, arms, .. } = self;

            let def_eq_pat = ctx
                .pat
                .take()
                .expect("internal error: there should be a pattern");
            let idents = def_eq_pat.get_idents();
            let when_pat = def_eq_pat
                .clone()
                .into_ir1(&mut ctx.rm_pat().add_gen(false))?;

            // idents initial values
            let init_stmts = init.map_or(Ok(vec![]), |init_arm| {
                init_arm.expr.check_is_constant(ctx.ctx0, ctx.errors)?;
                let loc = init_arm.expr.loc();
                let init_expr = init_arm.expr.into_ir1(ctx)?;
                let init_ids = idents
                    .into_iter()
                    .map(|(ident, _)| {
                        Ok((
                            ctx.ctx0.get_identifier_id(&ident, false, ctx.errors)?,
                            ctx.ctx0.get_init_id(&ident, false, ctx.errors)?,
                        ))
                    })
                    .collect::<TRes<HashMap<_, _>>>()?;
                let init_stmt = ir1::stream::InitStmt {
                    pattern: when_pat.clone(),
                    expr: init_expr,
                    init_ids,
                    loc,
                };
                Ok(vec![init_stmt])
            })?;

            // for each arm construct [ir1] pattern, guard and statements
            let mut new_arms = Vec::with_capacity(arms.len());
            for arm in arms.into_iter() {
                ctx.local();
                // transform event_pattern into [ir1] + set local context
                let event_pattern = arm.pattern.into_ir1(&mut ctx.rm_pat())?;
                // transform stmt_pat into [ir1] + set local context
                def_eq_pat.store(&mut ctx.rm_pat_loc())?;
                let stmt_pat = def_eq_pat
                    .clone()
                    .into_ir1(&mut ctx.rm_pat().add_gen(false))?;
                // transform AST guard into [ir1]
                let ir1_guard = arm.guard.map(|expr| expr.into_ir1(ctx)).transpose()?;
                // transform expression into [ir1] with local context
                let ir1_expr = arm.expr.into_ir1(ctx)?;
                // create stmt
                let loc = ir1_expr.loc();
                let stmt = ir1::Stmt {
                    pattern: stmt_pat,
                    expr: ir1_expr,
                    loc,
                };
                ctx.global();
                new_arms.push((event_pattern, ir1_guard, vec![stmt]))
            }

            // signals memory values
            let init_ids = {
                let mut init_ids = HashMap::with_capacity(init_stmts.len());
                for init_stmt in &init_stmts {
                    init_ids.extend(init_stmt.init_ids.clone());
                }
                init_ids
            };

            Ok((
                init_stmts,
                ir1::stream::Kind::when(when_pat, init_ids, new_arms),
            ))
        }
    }

    impl Ir0IntoIr1<ir1::ctx::PatLoc<'_>> for ir0::stream::Expr {
        type Ir1 = ir1::stream::Expr;

        // pre-condition: identifiers are stored in symbol table
        // post-condition: construct [ir1] stream expression and check identifiers good use
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc) -> TRes<Self::Ir1> {
            use ir1::stream::Kind;
            let kind = match self {
                stream::Expr::Application(app) => match *app.fun {
                    stream::Expr::Identifier(component) if ctx.is_comp(&component, false) => {
                        let called_comp_id = ctx.ctx0.get_comp_id(&component, false, ctx.errors)?;
                        let comp_symbol = ctx
                            .get_symbol(called_comp_id)
                            .expect("internal error: there should be a symbol")
                            .clone();
                        match comp_symbol.kind() {
                            SymbolKind::Component { inputs, .. } => {
                                // check inputs and comp_inputs have the same length
                                if inputs.len() != app.inputs.len() {
                                    bad!(ctx.errors, @ctx.loc => ErrorKind::arity_mismatch(
                                        app.inputs.len(), inputs.len()
                                    ))
                                }

                                Kind::call(
                                    called_comp_id,
                                    res_vec!(
                                        app.inputs.len(),
                                        app.inputs
                                            .into_iter()
                                            .zip(inputs)
                                            .map(|(input, id)| Ok((*id, input.into_ir1(ctx)?))),
                                    ),
                                )
                            }
                            _ => {
                                ctx.errors.push(error!(@comp_symbol.name().span() =>
                                    "fatal: symbol kind associated with component `{}` is not component-like",
                                    comp_symbol.name(),
                                ));
                                return Err(ErrorDetected);
                            }
                        }
                    }
                    fun => Kind::expr(ir1::expr::Kind::app(
                        fun.into_ir1(ctx)?,
                        res_vec!(
                            app.inputs.len(),
                            app.inputs
                                .into_iter()
                                .map(|input| input.clone().into_ir1(ctx)),
                        ),
                    )),
                },
                stream::Expr::Last(last) => {
                    let init_id = ctx.ctx0.get_init_id(&last.ident, false, ctx.errors)?;
                    let ident_id = ctx.ctx0.get_ident(&last.ident, false, false, ctx.errors)?;
                    Kind::last(init_id, ident_id)
                }
                stream::Expr::Emit(emit) => Kind::emit(emit.expr.into_ir1(ctx)?),
                stream::Expr::Constant(constant) => Kind::Expression {
                    expr: ir1::expr::Kind::Constant { constant },
                },
                stream::Expr::Identifier(id) => {
                    let id = ctx.ctx0.get_ident(&id, false, true, ctx.errors)?;
                    if let Some(value) = ctx.ctx0.try_get_const(id) {
                        let expr: ir0::stream::Expr =
                            value.clone().try_into().dewrap(ctx.errors)?;
                        return expr.into_ir1(ctx);
                    } else {
                        Kind::expr(ir1::expr::Kind::ident(id))
                    }
                }
                stream::Expr::UnOp(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::BinOp(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::IfThenElse(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::Lambda(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::Structure(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::Tuple(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::Enumeration(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::Array(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::MatchExpr(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::FieldAccess(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::TupleElementAccess(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::Map(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::Fold(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::Sort(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::Zip(expr) => Kind::expr(expr.into_ir1(ctx)?),
                stream::Expr::ArrayAccess(expr) => Kind::expr(expr.into_ir1(ctx)?),
            };

            Ok(ir1::stream::Expr::new(ctx.loc, kind))
        }
    }

    impl Ir0IntoIr1<ir1::ctx::PatLoc<'_>> for stream::ReactExpr {
        /// Reactive expressions `when` can contain initializations that are
        /// also part of their [ir1] representation, along the [ir1::stream::Expr].
        type Ir1 = (Vec<ir1::stream::InitStmt>, ir1::stream::Expr);

        // pre-condition: identifiers are stored in symbol table
        // post-condition: construct [ir1] stream expression and check identifiers good use
        fn into_ir1(self, ctx: &mut ir1::ctx::PatLoc) -> TRes<Self::Ir1> {
            match self {
                stream::ReactExpr::Expr(expr) => Ok((vec![], expr.into_ir1(ctx)?)),
                stream::ReactExpr::WhenExpr(expr) => {
                    let (init_stmts, kind) = expr.into_ir1(ctx)?;
                    Ok((init_stmts, ir1::stream::Expr::new(ctx.loc, kind)))
                }
            }
        }
    }
}

impl Ir0IntoIr1<ir1::ctx::WithLoc<'_>> for Typ {
    type Ir1 = Typ;

    /// Transforms AST into [ir1] and check identifiers good use.
    fn into_ir1(self, ctx: &mut ir1::ctx::WithLoc) -> TRes<Typ> {
        use syn::punctuated::{Pair, Punctuated};
        // pre-condition: Typedefs are stored in symbol table
        // post-condition: construct a new Type without `Typ::NotDefinedYet`
        match self {
                Typ::Array { bracket_token, ty, semi_token, size } => Ok(Typ::Array {
                    bracket_token,
                    ty: Box::new(ty.into_ir1(ctx)?),
                    semi_token,
                    size
                }),
                Typ::Tuple { paren_token, elements } => Ok(Typ::Tuple {
                    paren_token,
                    elements: elements
                        .into_pairs()
                        .map(|pair| {
                            let (ty, comma) = pair.into_tuple();
                            let ty = ty.into_ir1(ctx)?;
                            Ok(Pair::new(ty, comma))
                        }).collect::<TRes<_>>()?
                }),
                Typ::NotDefinedYet(name) => ctx
                    .get_struct_id(&name, false, ctx.loc, &mut vec![])
                    .map(|id| Typ::structure(name.clone(), id))
                    .or_else(|_| {
                        ctx
                            .get_enum_id(&name, false, ctx.loc, &mut vec![])
                            .map(|id| Typ::enumeration(name.clone(), id))
                    }).or_else(|_| {
                        let id = ctx.ctx0
                            .get_array_id(&name, false, ctx.loc, ctx.errors)?;
                        Ok(ctx.get_array(id))
                    }),
                Typ::Fn { paren_token, inputs, arrow_token, output } => {
                    let inputs = inputs.into_pairs()
                    .map(|pair| {
                        let (ty, comma) = pair.into_tuple();
                        let ty = ty.into_ir1(ctx)?;
                        Ok(Pair::new(ty, comma))
                    }).collect::<TRes<Punctuated<Typ, Token![,]>>>()?;
                    let output = output.into_ir1(ctx)?;
                    Ok(Typ::Fn { paren_token, inputs, arrow_token, output: output.into() })
                }
                Typ::Option { ty, question_token } => Ok(Typ::Option {
                    ty: Box::new(ty.into_ir1(ctx)?),
                    question_token
                }),
                Typ::Signal { signal_token, ty } => Ok(Typ::Signal {
                    signal_token,
                    ty: Box::new(ty.into_ir1(ctx)?),
                }),
                Typ::Event { event_token, ty } => Ok(Typ::Event {
                    event_token,
                    ty: Box::new(ty.into_ir1(ctx)?),
                }),
                Typ::Integer(_) | Typ::Float(_) | Typ::Boolean(_) | Typ::Unit(_) => Ok(self),
                Typ::Enumeration { .. }    // no enumeration at this time: they are `NotDefinedYet`
                | Typ::Structure { .. }    // no structure at this time: they are `NotDefinedYet`
                | Typ::Any                 // users cannot write `Any` type
                | Typ::Polymorphism(_)     // users cannot write `Polymorphism` type
                 => noErrorDesc!(),
            }
    }
}

impl Ir0IntoIr1<ir1::ctx::Simple<'_>> for ir0::Typedef {
    type Ir1 = ir1::Typedef;

    // pre-condition: typedefs are already stored in symbol table
    // post-condition: construct [ir1] typedef and check identifiers good use
    fn into_ir1(self, ctx: &mut ir1::ctx::Simple) -> TRes<Self::Ir1> {
        use ir0::Typedef;
        let loc = self.loc();
        match self {
            Typedef::Structure { ident, .. } => {
                let type_id = ctx.ctx0.get_struct_id(&ident, false, loc, ctx.errors)?;
                let field_ids = ctx.get_struct_fields(type_id).clone();

                Ok(ir1::Typedef {
                    id: type_id,
                    kind: ir1::typedef::Kind::Structure { fields: field_ids },
                    loc,
                })
            }

            Typedef::Enumeration { ident, .. } => {
                let type_id = ctx.ctx0.get_enum_id(&ident, false, loc, ctx.errors)?;
                let element_ids = ctx.get_enum_elements(type_id).clone();
                Ok(ir1::Typedef {
                    id: type_id,
                    kind: ir1::typedef::Kind::Enumeration {
                        elements: element_ids,
                    },
                    loc,
                })
            }

            Typedef::Array { ident, .. } => {
                let type_id = ctx.ctx0.get_array_id(&ident, false, loc, ctx.errors)?;
                Ok(ir1::Typedef {
                    id: type_id,
                    kind: ir1::typedef::Kind::Array,
                    loc,
                })
            }
        }
    }
}
