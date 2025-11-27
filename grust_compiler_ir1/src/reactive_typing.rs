prelude! { ctx::WithComp }

/// Performs reactive type analysis.
pub trait ReactiveTyping {
    /// Tries to reactive type the given construct.
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<Vec<ReactTyp>>;

    /// Get reactive type from construct.
    fn one_react_typ_check(&self, ctx: &mut WithComp) -> TRes<ReactTyp> {
        let react_typs = self.react_typ_check(ctx)?;
        debug_assert_eq!(1, react_typs.len());
        Ok(react_typs[0])
    }
}

impl File {
    pub fn react_typ_check(&self, ctx: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        let components: HashMap<_, _> = self
            .components
            .iter()
            .map(|comp| (comp.get_id(), comp))
            .collect();
        let functions: HashMap<_, _> = self
            .functions
            .iter()
            .map(|fun| (fun.get_id(), fun))
            .collect();
        let ctx = &mut ctx::WithComp::new(ctx, errors, &components, &functions);
        for import in self.interface.imports.values() {
            import.react_typ_check(ctx);
        }
        for export in self.interface.exports.values() {
            export.react_typ_check(ctx);
        }
        for service in &self.interface.services {
            // TODO: find something better (because here it works because the ids are ordered by
            // their creation order, which is the same as the statement order)
            let mut stmts = service.statements.iter().collect::<Vec<_>>();
            stmts.sort_by(|a, b| a.0.cmp(b.0));
            for (_, stmt) in stmts {
                stmt.react_typ_check(ctx)?;
            }
        }
        Ok(())
    }
}

impl ir1::interface::FlowImport {
    /// Set the reactive type of the given construct in the symbol table.
    fn react_typ_check(&self, ctx: &mut WithComp) {
        ctx.set_react_typ(self.id, ReactTyp::d())
    }
}

impl ir1::interface::FlowExport {
    /// Set the reactive type of the given construct in the symbol table.
    fn react_typ_check(&self, ctx: &mut WithComp) {
        ctx.set_react_typ(self.id, ReactTyp::d())
    }
}

impl interface::FlowStatement {
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<()> {
        use interface::*;
        match self {
            FlowStatement::Declaration(FlowDeclaration { pattern, expr, .. }) => {
                let react_typs = expr.react_typ_check(ctx)?;
                pattern.react_typ_check(&react_typs, ctx);
                Ok(())
            }
            FlowStatement::Instantiation(FlowInstantiation { pattern, expr, .. }) => {
                let ids = pattern.identifiers();
                let react_typs = expr.react_typ_check(ctx)?;
                debug_assert_eq!(ids.len(), react_typs.len());
                let expected_react_typs = ids.iter().map(|id| *ctx.ctx0.get_react_typ(*id));
                expected_react_typs
                    .zip(react_typs)
                    .try_for_each(|(expected, react_typ)| {
                        react_typ.check_eq(expected, pattern.loc).dewrap(ctx.errors)
                    })
            }
        }
    }
}

impl ReactiveTyping for flow::Expr {
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<Vec<ReactTyp>> {
        match &self.kind {
            flow::Kind::Ident { id } => {
                let react_typ = ctx.get_react_typ(*id);
                Ok(vec![*react_typ])
            }
            flow::Kind::Time { .. } => Ok(vec![ReactTyp::c()]),
            flow::Kind::Sample { expr, .. }
            | flow::Kind::Timeout { expr, .. }
            | flow::Kind::Throttle { expr, .. }
            | flow::Kind::OnChange { expr }
            | flow::Kind::Persist { expr } => match expr.one_react_typ_check(ctx)? {
                ReactTyp::Discrete => Ok(vec![ReactTyp::d()]),
                given_react_type => {
                    bad!(ctx.errors, @expr.loc => ErrorKind::not_reactive(given_react_type, ReactTyp::d()))
                }
            },
            flow::Kind::Scan { expr, .. } => {
                expr.react_typ_check(ctx)?;
                Ok(vec![ReactTyp::d()])
            }
            flow::Kind::Merge { expr_1, expr_2, .. } => match expr_1.one_react_typ_check(ctx)? {
                ReactTyp::Discrete => match expr_2.one_react_typ_check(ctx)? {
                    ReactTyp::Discrete => Ok(vec![ReactTyp::d()]),
                    given_react_type => {
                        bad!(ctx.errors, @expr_2.loc => ErrorKind::not_reactive(given_react_type, ReactTyp::d()))
                    }
                },
                given_react_type => {
                    bad!(ctx.errors, @expr_1.loc => ErrorKind::not_reactive(given_react_type, ReactTyp::d()))
                }
            },
            flow::Kind::Period { .. } => Ok(vec![ReactTyp::d()]),
            flow::Kind::SampleOn { expr, event } => match expr.one_react_typ_check(ctx)? {
                ReactTyp::Discrete => match event.one_react_typ_check(ctx)? {
                    ReactTyp::Discrete => Ok(vec![ReactTyp::d()]),
                    given_react_type => {
                        bad!(ctx.errors, @event.loc => ErrorKind::not_reactive(given_react_type, ReactTyp::d()))
                    }
                },
                given_react_type => {
                    bad!(ctx.errors, @expr.loc => ErrorKind::not_reactive(given_react_type, ReactTyp::d()))
                }
            },
            flow::Kind::ScanOn { expr, event } => {
                expr.react_typ_check(ctx)?;
                match event.one_react_typ_check(ctx)? {
                    ReactTyp::Discrete => Ok(vec![ReactTyp::d()]),
                    given_react_type => {
                        bad!(ctx.errors, @event.loc => ErrorKind::not_reactive(given_react_type, ReactTyp::d()))
                    }
                }
            }

            flow::Kind::ComponentCall {
                ref called_comp_id,
                ref inputs,
                ..
            } => {
                // type all inputs and set their types
                inputs.iter().try_for_each(|(id, input)| {
                    let react_typ = input.one_react_typ_check(ctx)?;
                    ctx.set_react_typ(*id, react_typ);
                    Ok(())
                })?;

                let called_comp = ctx
                    .components
                    .get(called_comp_id)
                    .expect("internal error: there should be a component");

                // compute the outputs reactive types
                let outputs_react_typs = called_comp.react_typ_check(ctx)?;

                // reset the reactive type of all identifiers in the component
                called_comp.reset_react_typs(ctx);
                inputs.iter().for_each(|(id, _)| ctx.reset_react_typ(*id));

                Ok(outputs_react_typs)
            }
            flow::Kind::FunctionCall {
                ref function_id,
                ref inputs,
                ..
            } => {
                // type all inputs and set their types
                inputs.iter().try_for_each(|(id, input)| {
                    let react_typ = input.one_react_typ_check(ctx)?;
                    ctx.set_react_typ(*id, react_typ);
                    Ok(())
                })?;

                let called_fun = ctx
                    .functions
                    .get(function_id)
                    .expect("internal error: there should be a function");

                // compute the output reactive type
                let outputs_react_typ = called_fun.react_typ_check(ctx)?;

                // reset the reactive type of all identifiers in the function
                called_fun.reset_react_typs(ctx);
                inputs.iter().for_each(|(id, _)| ctx.reset_react_typ(*id));

                Ok(vec![outputs_react_typ])
            }
        }
    }
}

impl Function {
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<ReactTyp> {
        if let Either::Left(body) = &self.body_or_path {
            for stmt in body.statements.iter() {
                stmt.react_typ_check(ctx)?;
            }
            let output_react_typ = body.returned.one_react_typ_check(ctx)?;
            Ok(output_react_typ)
        } else {
            Ok(ReactTyp::d())
        }
    }

    fn reset_react_typs(&self, ctx: &mut WithComp) {
        if let Either::Left(fun_body) = &self.body_or_path {
            for stmt in &fun_body.statements {
                for id in stmt.pattern.identifiers() {
                    ctx.reset_react_typ(id);
                }
            }
        }
    }
}

impl Component {
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<Vec<ReactTyp>> {
        if let Either::Left(mut comp_body) = self.body_or_path.clone() {
            // TODO: find better
            comp_body.schedule();
            comp_body.react_typ_check(ctx)?;
            Ok(ctx
                .get_comp_outputs(self.sign.id)
                .iter()
                .map(|(_, output_id)| *ctx.get_react_typ(*output_id))
                .collect::<Vec<_>>())
        } else {
            Ok(ctx
                .get_comp_outputs(self.sign.id)
                .iter()
                .map(|(_, _)| ReactTyp::d())
                .collect::<Vec<_>>())
        }
    }

    fn reset_react_typs(&self, ctx: &mut WithComp) {
        if let Either::Left(comp_body) = &self.body_or_path {
            let ids = comp_body.get_idents_id();
            // ids.sort();
            // ids.dedup();
            for id in ids {
                ctx.reset_react_typ(id);
            }
            for init in &comp_body.inits {
                for id in init.init_ids.values() {
                    ctx.reset_react_typ(*id);
                }
            }
        }
    }
}

impl ComponentBody {
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<()> {
        for init in self.inits.iter() {
            init.react_typ_last(ctx);
        }
        for stmt in self.statements.iter() {
            stmt.react_typ_check(ctx)?;
        }
        for init in self.inits.iter() {
            init.react_typ_check(ctx)?;
        }
        Ok(())
    }
}

impl stream::InitStmt {
    fn react_typ_last(&self, ctx: &mut WithComp) {
        self.init_ids
            .iter()
            .for_each(|(_, last_id)| ctx.set_react_typ(*last_id, ReactTyp::c()))
    }
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<()> {
        self.init_ids.iter().try_for_each(|(id, _)| {
            ctx.get_react_typ(*id)
                .check_eq(ReactTyp::d(), self.loc)
                .dewrap(ctx.errors)
        })
    }
}

impl<E: ReactiveTyping> Stmt<E> {
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<()> {
        let react_typs = self.expr.react_typ_check(ctx)?;
        self.pattern.react_typ_check(&react_typs, ctx);
        Ok(())
    }
}

impl stmt::Pattern {
    /// Tries to construct the type of the given construct.
    fn react_typ_check(&self, expected_react_typs: &Vec<ReactTyp>, ctx: &mut WithComp) {
        match self.kind {
            stmt::Kind::Identifier { id } | stmt::Kind::Typed { id, .. } => {
                debug_assert!(expected_react_typs.len() == 1);
                let react_typ = expected_react_typs[0];
                ctx.set_react_typ(id, react_typ);
            }
            stmt::Kind::Tuple { ref elements } => {
                debug_assert!(expected_react_typs.len() == elements.len());
                elements
                    .iter()
                    .zip(expected_react_typs)
                    .for_each(|(pat, react_typ)| pat.react_typ_check(&vec![*react_typ], ctx));
            }
        }
    }
}

impl ReactiveTyping for stream::Expr {
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<Vec<ReactTyp>> {
        match self.kind {
            stream::Kind::Last { init_id, .. } => {
                let react_typ = *ctx.get_react_typ(init_id);
                react_typ
                    .check_eq(ReactTyp::c(), self.loc)
                    .dewrap(ctx.errors)?;
                Ok(vec![react_typ])
            }
            stream::Kind::SomeEvent { .. } => Ok(vec![ReactTyp::c()]),
            stream::Kind::Expression { ref expr } => expr.react_typ_check(self.loc, ctx),
            stream::Kind::WhenExpr {
                ref def_eq_pat,
                ref arms,
                ..
            } => {
                arms.iter().try_for_each(|(pattern, _, stmts)| {
                    // type pattern
                    pattern.react_typ_check(ctx)?;
                    // the guard is not checked: it can be anything
                    // make all identifers in stmt discrete
                    stmts.iter().for_each(|stmt| {
                        stmt.get_identifiers()
                            .iter()
                            .for_each(|id| ctx.set_react_typ(*id, ReactTyp::d()))
                    });
                    Ok(())
                })?;

                let react_typs = vec![ReactTyp::d(); def_eq_pat.identifiers().len()];
                Ok(react_typs)
            }
            stream::Kind::ComponentApplication {
                called_comp_id,
                ref inputs,
                ..
            } => {
                // type all inputs and set their types
                inputs.iter().try_for_each(|(id, input)| {
                    let react_typ = input.one_react_typ_check(ctx)?;
                    ctx.set_react_typ(*id, react_typ);
                    Ok(())
                })?;

                let called_comp = ctx
                    .components
                    .get(&called_comp_id)
                    .expect("internal error: there should be a component");

                // compute the outputs reactive types
                let outputs_react_typs = called_comp.react_typ_check(ctx)?;

                // reset the reactive type of all identifiers in the component
                called_comp.reset_react_typs(ctx);
                inputs.iter().for_each(|(id, _)| ctx.reset_react_typ(*id));

                Ok(outputs_react_typs)
            }
            stream::Kind::RisingEdge { ref expr } => {
                // check expr is a boolean
                let expr_type = expr.one_react_typ_check(ctx)?;
                let expected = ReactTyp::d();
                expr_type.check_eq(expected, self.loc).dewrap(ctx.errors)?;
                // set the type
                Ok(vec![expected])
            }
            stream::Kind::NoneEvent => unreachable!("react_typ_check in expressions"),
        }
    }
}

impl ReactiveTyping for Expr {
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<Vec<ReactTyp>> {
        self.kind.react_typ_check(self.loc, ctx)
    }
}

pub trait IsExpr {
    fn identifier(&self) -> Option<usize>;
}
impl IsExpr for Expr {
    fn identifier(&self) -> Option<usize> {
        match &self.kind {
            expr::Kind::Identifier { id } => Some(*id),
            _ => None,
        }
    }
}
impl IsExpr for stream::Expr {
    fn identifier(&self) -> Option<usize> {
        match &self.kind {
            stream::Kind::Expression {
                expr: expr::Kind::Identifier { id },
            } => Some(*id),
            _ => None,
        }
    }
}

impl<E: ReactiveTyping + IsExpr> expr::Kind<E> {
    /// Tries to type the given construct.
    fn react_typ_check(&self, loc: Loc, ctx: &mut WithComp) -> TRes<Vec<ReactTyp>> {
        match self {
            expr::Kind::Constant { constant } => Ok(vec![constant.get_react_typ()]),
            expr::Kind::Identifier { id } => Ok(vec![*ctx.get_react_typ(*id)]),
            expr::Kind::UnOp { expr, .. }
            | expr::Kind::ArrayAccess { expr, .. }
            | expr::Kind::TupleElementAccess { expr, .. }
            | expr::Kind::FieldAccess { expr, .. } => Ok(vec![expr.one_react_typ_check(ctx)?]),
            expr::Kind::BinOp { lft, rgt, .. } => {
                match (lft.one_react_typ_check(ctx)?, rgt.one_react_typ_check(ctx)?) {
                    (ReactTyp::Continuous, _) | (_, ReactTyp::Continuous) => {
                        Ok(vec![ReactTyp::c()])
                    }
                    (ReactTyp::Discrete, ReactTyp::Discrete) => Ok(vec![ReactTyp::d()]),
                }
            }
            expr::Kind::IfThenElse { cnd, thn, els } => {
                match (
                    cnd.one_react_typ_check(ctx)?,
                    thn.one_react_typ_check(ctx)?,
                    els.one_react_typ_check(ctx)?,
                ) {
                    (ReactTyp::Continuous, _, _)
                    | (_, ReactTyp::Continuous, _)
                    | (_, _, ReactTyp::Continuous) => Ok(vec![ReactTyp::c()]),
                    (ReactTyp::Discrete, ReactTyp::Discrete, ReactTyp::Discrete) => {
                        Ok(vec![ReactTyp::d()])
                    }
                }
            }
            expr::Kind::Enumeration { .. } => Ok(vec![ReactTyp::d()]),
            expr::Kind::Lambda { inputs, expr, .. } => {
                // set input to discrete
                inputs
                    .iter()
                    .for_each(|id| ctx.set_react_typ(*id, ReactTyp::d()));
                // the expression should be discrete
                expr.one_react_typ_check(ctx)?
                    .check_eq(ReactTyp::d(), loc)
                    .dewrap(ctx.errors)?;
                // reset input reactive type
                inputs.iter().for_each(|id| ctx.reset_react_typ(*id));
                Ok(vec![ReactTyp::d()])
            }
            expr::Kind::Application { fun, inputs } => {
                match fun.identifier() {
                    Some(function_id) => {
                        let called_fun = ctx
                            .functions
                            .get(&function_id)
                            .expect("internal error: there should be a function");

                        let input_ids = ctx.get_function_input(function_id).clone();

                        // type all inputs and set their types
                        input_ids.iter().zip(inputs).try_for_each(|(id, input)| {
                            let react_typ = input.one_react_typ_check(ctx)?;
                            ctx.set_react_typ(*id, react_typ);
                            Ok(())
                        })?;

                        // compute the output reactive type
                        let outputs_react_typ = called_fun.react_typ_check(ctx)?;

                        // reset the reactive type of all identifiers in the function
                        called_fun.reset_react_typs(ctx);
                        input_ids.iter().for_each(|id| ctx.reset_react_typ(*id));

                        Ok(vec![outputs_react_typ])
                    }
                    _ => {
                        // the function should be discrete
                        fun.one_react_typ_check(ctx)?
                            .check_eq(ReactTyp::d(), loc)
                            .dewrap(ctx.errors)?;
                        let react_typs = inputs
                            .iter()
                            .map(|expr| expr.one_react_typ_check(ctx))
                            .collect::<TRes<Vec<_>>>()?;
                        // if one input is continuous, then it is continous
                        if react_typs
                            .iter()
                            .any(|react_typ| *react_typ == ReactTyp::c())
                        {
                            Ok(vec![ReactTyp::c()])
                        } else {
                            Ok(vec![ReactTyp::d()])
                        }
                    }
                }
            }
            expr::Kind::Structure { fields, .. } => {
                let react_typs = fields
                    .iter()
                    .map(|(_, expr)| expr.one_react_typ_check(ctx))
                    .collect::<TRes<Vec<_>>>()?;
                // if one input is continuous, then it is continous
                if react_typs
                    .iter()
                    .any(|react_typ| *react_typ == ReactTyp::c())
                {
                    Ok(vec![ReactTyp::c()])
                } else {
                    Ok(vec![ReactTyp::d()])
                }
            }
            expr::Kind::Array { elements } | expr::Kind::Zip { arrays: elements } => {
                let react_typs = elements
                    .iter()
                    .map(|expr| expr.one_react_typ_check(ctx))
                    .collect::<TRes<Vec<_>>>()?;
                // if one input is continuous, then it is continous
                if react_typs
                    .iter()
                    .any(|react_typ| *react_typ == ReactTyp::c())
                {
                    Ok(vec![ReactTyp::c()])
                } else {
                    Ok(vec![ReactTyp::d()])
                }
            }
            expr::Kind::Tuple { elements } => elements
                .iter()
                .map(|expr| expr.one_react_typ_check(ctx))
                .collect::<TRes<Vec<_>>>(),
            expr::Kind::Map { expr, fun } | expr::Kind::Sort { expr, fun } => {
                // the function should be discrete
                fun.one_react_typ_check(ctx)?
                    .check_eq(ReactTyp::d(), loc)
                    .dewrap(ctx.errors)?;
                Ok(vec![expr.one_react_typ_check(ctx)?])
            }
            expr::Kind::Fold { array, init, fun } => {
                // the function should be discrete
                fun.one_react_typ_check(ctx)?
                    .check_eq(ReactTyp::d(), loc)
                    .dewrap(ctx.errors)?;
                // if one is continuous, then it is continous
                if array.one_react_typ_check(ctx)? == ReactTyp::c()
                    || init.one_react_typ_check(ctx)? == ReactTyp::c()
                {
                    Ok(vec![ReactTyp::c()])
                } else {
                    Ok(vec![ReactTyp::d()])
                }
            }
            expr::Kind::MatchExpr { expr, arms } => {
                let react_typs = expr.react_typ_check(ctx)?;
                let mut match_react_typs = vec![];
                for (pat, guard, stmts, expr) in arms {
                    pat.react_typ_check(&react_typs, ctx)?;

                    if let Some(guard) = guard {
                        guard
                            .one_react_typ_check(ctx)?
                            .check_eq(ReactTyp::d(), loc)
                            .dewrap(ctx.errors)?;
                    }

                    stmts
                        .iter()
                        .try_for_each(|stmt| stmt.react_typ_check(ctx))?;

                    let expr_react_typs = expr.react_typ_check(ctx)?;
                    if match_react_typs.is_empty() {
                        match_react_typs = expr_react_typs.clone();
                    } else {
                        debug_assert_eq!(expr_react_typs.len(), match_react_typs.len());
                        match_react_typs = match_react_typs
                            .into_iter()
                            .zip(expr_react_typs)
                            .map(|(rt1, rt2)| match (rt1, rt2) {
                                (ReactTyp::Continuous, _) | (_, ReactTyp::Continuous) => {
                                    ReactTyp::c()
                                }
                                (ReactTyp::Discrete, ReactTyp::Discrete) => ReactTyp::d(),
                            })
                            .collect();
                    }
                }
                Ok(match_react_typs)
            }
        }
    }
}

impl Pattern {
    /// Tries to type the given construct.
    fn react_typ_check(&self, expected_react_typs: &Vec<ReactTyp>, ctx: &mut WithComp) -> TRes<()> {
        use pattern::Kind;
        match self.kind {
            Kind::Constant { .. } | Kind::Enumeration { .. } => {
                debug_assert!(expected_react_typs.len() == 1);
                expected_react_typs[0]
                    .check_eq(ReactTyp::d(), self.loc)
                    .dewrap(ctx.errors)?;
                Ok(())
            }
            Kind::Identifier { id } => {
                debug_assert!(expected_react_typs.len() == 1);
                ctx.set_react_typ(id, expected_react_typs[0]);
                Ok(())
            }
            Kind::Structure { ref fields, .. } => {
                debug_assert!(expected_react_typs.len() == 1);
                let react_typ = expected_react_typs[0];
                fields.iter().try_for_each(|(id, optional_pattern)| {
                    if let Some(pattern) = optional_pattern {
                        pattern.react_typ_check(expected_react_typs, ctx)?;
                    } else {
                        ctx.set_react_typ(*id, react_typ);
                    }
                    Ok(())
                })?;
                Ok(())
            }
            Kind::Tuple { ref elements } => {
                debug_assert!(expected_react_typs.len() == elements.len());
                elements
                    .iter()
                    .zip(expected_react_typs)
                    .try_for_each(|(pat, react_typ)| pat.react_typ_check(&vec![*react_typ], ctx))
            }
            Kind::Default(_) => Ok(()),
            Kind::PresentEvent { .. } => unreachable!("react_typ_check in patterns"),
        }
    }
}

impl EventPattern {
    /// Tries to type the given construct.
    fn react_typ_check(&self, ctx: &mut WithComp) -> TRes<()> {
        use event_pattern::Kind;
        match self.kind {
            Kind::Tuple { ref elements } => elements
                .iter()
                .try_for_each(|epat| epat.react_typ_check(ctx)),
            Kind::Let {
                ref pattern,
                ref event_id,
            } => match ctx.get_react_typ(*event_id) {
                ReactTyp::Discrete => {
                    pattern
                        .identifiers()
                        .iter()
                        .for_each(|id| ctx.set_react_typ(*id, ReactTyp::d()));
                    Ok(())
                }
                react_typ => Err(
                    error!(@self.loc => ErrorKind::not_reactive(*react_typ, ReactTyp::Discrete)),
                )
                .dewrap(ctx.errors),
            },
            Kind::RisingEdge(ref expr) => {
                expr.one_react_typ_check(ctx)?
                    .check_eq(ReactTyp::d(), self.loc)
                    .dewrap(ctx.errors)?;
                Ok(())
            }
        }
    }
}
