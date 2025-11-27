prelude! {}

/// Performs type analysis.
pub trait Typing {
    /// Tries to type the given construct.
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()>;

    /// Get type from construct.
    fn get_typ(&self) -> Option<&Typ> {
        None
    }

    /// Get mutable type from construct.
    fn get_typ_mut(&mut self) -> Option<&mut Typ> {
        None
    }
}

impl Typing for File {
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        for c in self.components.iter_mut() {
            c.typ_check(symbols, errors)?;
        }
        for f in self.functions.iter_mut() {
            f.typ_check(symbols, errors)?;
        }
        for s in self.interface.services.iter_mut() {
            for stmt in s.statements.values_mut() {
                stmt.typ_check(symbols, errors)?;
            }
        }
        Ok(())
    }
}

impl Typing for Function {
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        if let Either::Left(body) = self.body_or_path.as_mut() {
            for stmt in body.statements.iter_mut() {
                stmt.typ_check(symbols, errors)?;
            }
            body.returned.typ_check(symbols, errors)?;
            let expected_type = symbols.get_function_output_type(self.id);
            body.returned
                .get_typ()
                .expect("internal error: expression should be typed")
                .expect(self.loc, expected_type)
                .dewrap(errors)
        } else {
            Ok(())
        }
    }
}

impl Typing for Component {
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        if let Either::Left(comp_body) = &mut self.body_or_path {
            comp_body.typ_check(symbols, errors)
        } else {
            Ok(())
        }
    }
}

impl Typing for ComponentBody {
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        for init in self.inits.iter_mut() {
            init.typ_check(symbols, errors)?;
        }
        for stmt in self.statements.iter_mut() {
            stmt.typ_check(symbols, errors)?;
        }
        self.contract.typ_check(symbols, errors)
    }
}

impl Typing for Contract {
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        for term in self.requires.iter_mut() {
            term.typ_check(symbols, errors)?;
        }
        for term in self.ensures.iter_mut() {
            term.typ_check(symbols, errors)?;
        }
        for term in self.invariant.iter_mut() {
            term.typ_check(symbols, errors)?;
        }
        Ok(())
    }
}

impl Typing for contract::Term {
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        let ty = match &mut self.kind {
            contract::Kind::Constant { constant } => constant.get_typ(),
            contract::Kind::Identifier { id } => symbols.get_typ(*id).clone(),
            contract::Kind::Last { init_id, ident_id } => {
                let ident_type = symbols.get_typ(*ident_id).clone();
                let init_type = symbols.get_typ(*init_id);
                check::typ::expect(self.loc, init_type, &ident_type).map_err(|_| ErrorDetected)?;
                ident_type
            }
            contract::Kind::Enumeration { enum_id, .. } => Typ::Enumeration {
                name: symbols.get_name(*enum_id).clone(),
                id: *enum_id,
            },
            contract::Kind::Paren { term } => {
                term.typ_check(symbols, errors)?;
                term.typing.as_ref().unwrap().clone()
            }
            contract::Kind::Unary { op, term } => {
                term.typ_check(symbols, errors)?;
                let ty = term.typing.as_ref().unwrap().clone();
                let mut unop_type = op.get_typ();
                unop_type.apply(vec![ty], self.loc, errors)?
            }
            contract::Kind::Binary { op, left, right } => {
                left.typ_check(symbols, errors)?;
                let left_type = left.typing.as_ref().unwrap().clone();
                right.typ_check(symbols, errors)?;
                let right_type = right.typing.as_ref().unwrap().clone();
                let mut binop_type = op.get_typ();
                binop_type.apply(vec![left_type, right_type], self.loc, errors)?
            }
            contract::Kind::ForAll { term, .. } => {
                term.typ_check(symbols, errors)?;
                let ty = term.typing.as_ref().unwrap();
                ty.expect_bool(self.loc).dewrap(errors)?;
                Typ::bool()
            }
            contract::Kind::Implication { left, right } => {
                left.typ_check(symbols, errors)?;
                let ty = left.typing.as_ref().unwrap();
                ty.expect_bool(self.loc).dewrap(errors)?;
                right.typ_check(symbols, errors)?;
                let ty = right.typing.as_ref().unwrap();
                ty.expect_bool(self.loc).dewrap(errors)?;
                ty.clone()
            }
            contract::Kind::PresentEvent { event_id, pattern } => {
                let typing = symbols.get_typ(*event_id).clone();
                match &typing {
                    Typ::Option { ty, .. } => {
                        symbols.set_type(*pattern, *ty.clone());
                    }
                    ty => Err(error!(@self.loc => ErrorKind::expected_event(ty.clone())))
                        .dewrap(errors)?,
                }
                typing
            }
            contract::Kind::Application { fun_id, inputs } => {
                // type all inputs
                for input in inputs.iter_mut() {
                    input.typ_check(symbols, errors)?;
                }

                let input_types = inputs
                    .iter()
                    .map(|input| input.get_typ().unwrap().clone())
                    .collect::<Vec<_>>();

                // compute the application type
                let mut fun_ty = symbols.get_typ(*fun_id).clone();
                fun_ty.apply(input_types, self.loc, errors)?
            }

            contract::Kind::ComponentCall {
                comp_id, inputs, ..
            } => {
                // type all inputs and check their types
                inputs.iter_mut().try_for_each(|(id, input)| {
                    input.typ_check(symbols, errors)?;

                    let input_type = input.typing.as_ref().unwrap();
                    let expected_type = symbols.get_typ(*id);
                    input_type.expect(self.loc, expected_type).dewrap(errors)
                })?;

                // get the called ident type
                let comp_call_type = {
                    let mut outputs_types = symbols
                        .get_comp_outputs(*comp_id)
                        .iter()
                        .map(|(_, output_ident)| symbols.get_typ(*output_ident).clone())
                        .collect::<Vec<_>>();
                    if outputs_types.len() == 1 {
                        outputs_types.pop().unwrap()
                    } else {
                        Typ::tuple(outputs_types)
                    }
                };

                comp_call_type
            }
        };
        self.typing = Some(ty);
        Ok(())
    }

    fn get_typ(&self) -> Option<&Typ> {
        self.typing.as_ref()
    }

    fn get_typ_mut(&mut self) -> Option<&mut Typ> {
        self.typing.as_mut()
    }
}

impl Typing for interface::FlowStatement {
    // pre-condition: identifiers associated with statement is already typed
    // post-condition: expression associated with statement is typed and checked
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        use interface::*;
        match self {
            FlowStatement::Declaration(FlowDeclaration {
                let_token,
                semi_token,
                pattern,
                expr,
                ..
            }) => {
                let expected_type = pattern.typ.as_ref().unwrap();
                expr.typ_check(symbols, errors)?;
                let expression_type = expr.get_typ().unwrap();
                let loc = let_token
                    .span
                    .join(semi_token.span)
                    .map(Loc::from)
                    .unwrap_or(pattern.loc);
                expression_type.expect(loc, expected_type).dewrap(errors)
            }
            FlowStatement::Instantiation(FlowInstantiation {
                pattern,
                eq_token,
                semi_token,
                expr,
            }) => {
                pattern.typ_check(symbols, errors)?;
                let expected_type = pattern.typ.as_ref().unwrap();
                expr.typ_check(symbols, errors)?;
                let expression_type = expr.get_typ().unwrap();
                let loc = pattern
                    .loc
                    .try_join(semi_token.span)
                    .unwrap_or(eq_token.span.into());
                expression_type.expect(loc, expected_type).dewrap(errors)
            }
        }
    }
}

impl Typing for flow::Expr {
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        match &mut self.kind {
            flow::Kind::Ident { id } => {
                self.typ = Some(symbols.get_typ(*id).clone());
                Ok(())
            }
            flow::Kind::Sample { expr, .. } => {
                expr.typ_check(symbols, errors)?;
                // get expression type
                match expr.get_typ().unwrap() {
                    Typ::Event { ty: typ, .. } => {
                        // set typing
                        self.typ = Some(Typ::event((**typ).clone()));
                        Ok(())
                    }
                    given_type => {
                        bad!(errors, @expr.loc => ErrorKind::expected_event(given_type.clone()))
                    }
                }
            }
            flow::Kind::Scan { expr, .. } => {
                expr.typ_check(symbols, errors)?;
                // get expression type
                match expr.get_typ().unwrap() {
                    Typ::Signal { ty: typ, .. } => {
                        // set typ
                        self.typ = Some(Typ::signal((**typ).clone()));
                        Ok(())
                    }
                    given_type => {
                        bad!(errors, @expr.loc => ErrorKind::expected_signal(given_type.clone()))
                    }
                }
            }
            flow::Kind::Timeout { expr, .. } => {
                expr.typ_check(symbols, errors)?;
                // get expression type
                match expr.get_typ().unwrap() {
                    Typ::Event { .. } => (),
                    given_type => {
                        bad!(errors, @expr.loc => ErrorKind::expected_event(given_type.clone()));
                    }
                }
                // set typing
                self.typ = Some(Typ::event(Typ::unit()));
                Ok(())
            }
            flow::Kind::Throttle { expr, delta } => {
                expr.typ_check(symbols, errors)?;
                // get expression type
                match expr.get_typ().unwrap() {
                    Typ::Signal { ty: typ, .. } => {
                        let delta_ty = delta.get_typ();
                        typ.expect(expr.loc, &delta_ty).dewrap(errors)?;
                        // set typing
                        self.typ = Some(Typ::signal((**typ).clone()));
                        Ok(())
                    }
                    given_type => {
                        bad!(errors, @expr.loc => ErrorKind::expected_signal(given_type.clone()))
                    }
                }
            }
            flow::Kind::OnChange { expr } => {
                expr.typ_check(symbols, errors)?;
                // get expression type
                match expr.get_typ().unwrap() {
                    Typ::Signal { ty: typ, .. } => {
                        // set typing
                        self.typ = Some(Typ::event((**typ).clone()));
                        Ok(())
                    }
                    given_type => {
                        bad!(errors, @expr.loc => ErrorKind::expected_signal(given_type.clone()))
                    }
                }
            }
            flow::Kind::Persist { expr } => {
                expr.typ_check(symbols, errors)?;
                // get expression type
                match expr.get_typ().unwrap() {
                    Typ::Event { ty: typ, .. } => {
                        // set typing
                        self.typ = Some(Typ::signal((**typ).clone()));
                        Ok(())
                    }
                    given_type => {
                        bad!(errors, @expr.loc => ErrorKind::expected_event(given_type.clone()))
                    }
                }
            }
            flow::Kind::Merge { expr_1, expr_2, .. } => {
                expr_1.typ_check(symbols, errors)?;
                expr_2.typ_check(symbols, errors)?;
                // get expression type
                match expr_1.get_typ().unwrap() {
                    Typ::Event { ty: typ_1, .. } => {
                        match expr_2.get_typ().unwrap() {
                            Typ::Event { ty: typ_2, .. } => {
                                typ_2.expect(expr_2.loc, typ_1).dewrap(errors)?;
                                // set typing
                                self.typ = Some(Typ::event((**typ_1).clone()));
                                Ok(())
                            }
                            given_type => {
                                bad!(errors, @expr_2.loc => ErrorKind::expected_event(given_type.clone()))
                            }
                        }
                    }
                    given_type => {
                        bad!(errors, @expr_1.loc => ErrorKind::expected_event(given_type.clone()))
                    }
                }
            }
            flow::Kind::Time { .. } => {
                self.typ = Some(Typ::signal(Typ::float()));
                Ok(())
            }
            flow::Kind::Period { .. } => {
                self.typ = Some(Typ::event(Typ::float()));
                Ok(())
            }
            flow::Kind::SampleOn { expr, event } => {
                expr.typ_check(symbols, errors)?;
                event.typ_check(symbols, errors)?;
                // get expression type
                match expr.get_typ().unwrap() {
                    Typ::Event { ty: typ, .. } => {
                        // set typing
                        self.typ = Some(Typ::event((**typ).clone()));
                        // check 'event' is an event
                        match event.get_typ().unwrap() {
                            Typ::Event { .. } => Ok(()),
                            given_type => {
                                bad!(errors, @event.loc => ErrorKind::expected_event(given_type.clone()))
                            }
                        }
                    }
                    given_type => {
                        bad!(errors, @expr.loc => ErrorKind::expected_event(given_type.clone()))
                    }
                }
            }
            flow::Kind::ScanOn { expr, event } => {
                expr.typ_check(symbols, errors)?;
                event.typ_check(symbols, errors)?;
                // get expression type
                match expr.get_typ().unwrap() {
                    Typ::Signal { ty: typ, .. } => {
                        // set typ
                        self.typ = Some(Typ::signal((**typ).clone()));
                        // check 'event' is an event
                        match event.get_typ().unwrap() {
                            Typ::Event { .. } => Ok(()),
                            given_type => {
                                bad!(errors, @event.loc => ErrorKind::expected_event(given_type.clone()))
                            }
                        }
                    }
                    given_type => {
                        bad!(errors, @expr.loc => ErrorKind::expected_signal(given_type.clone()))
                    }
                }
            }
            flow::Kind::ComponentCall {
                ref called_comp_id,
                ref mut inputs,
                ..
            } => {
                // type all inputs and check their types
                inputs.iter_mut().try_for_each(|(id, input)| {
                    input.typ_check(symbols, errors)?;
                    let input_type = input.get_typ().unwrap().convert();
                    let expected_type = symbols.get_typ(*id);
                    input_type.expect(self.loc, expected_type).dewrap(errors)
                })?;

                // get the outputs types of the called component
                let mut outputs_types = symbols
                    .get_comp_outputs(*called_comp_id)
                    .iter()
                    .map(|(_, output_id)| {
                        let output_type = symbols.get_typ(*output_id);
                        output_type.rev_convert()
                    })
                    .collect::<Vec<_>>();

                // construct component application type
                let comp_application_type = if outputs_types.len() == 1 {
                    outputs_types.pop().unwrap()
                } else {
                    Typ::tuple(outputs_types)
                };

                self.typ = Some(comp_application_type);
                Ok(())
            }
            flow::Kind::FunctionCall {
                ref function_id,
                ref mut inputs,
                ..
            } => {
                // type all inputs and check their types
                inputs.iter_mut().try_for_each(|(id, input)| {
                    input.typ_check(symbols, errors)?;
                    let input_type = input.get_typ().unwrap().convert();
                    let expected_type = symbols.get_typ(*id);
                    input_type.expect(self.loc, expected_type).dewrap(errors)
                })?;

                // get the output type of the called function
                let outputs_type = symbols.get_function_output_type(*function_id);

                self.typ = Some(outputs_type.rev_convert());
                Ok(())
            }
        }
    }

    fn get_typ(&self) -> Option<&Typ> {
        self.typ.as_ref()
    }

    fn get_typ_mut(&mut self) -> Option<&mut Typ> {
        self.typ.as_mut()
    }
}

impl Typing for stream::Expr {
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        match self.kind {
            stream::Kind::Last { ident_id, .. } => {
                // check the scope is not 'very_local'
                let sym = symbols.resolve_symbol(self.loc, ident_id).dewrap(errors)?;
                if sym
                    .kind()
                    .scope()
                    .map(|s| s == &Scope::VeryLocal)
                    .unwrap_or(false)
                {
                    bad!(errors, @sym.loc() =>
                        "`{}` has an unexpected `VeryLocal` scope", sym.name()
                    );
                }

                let id_type = symbols.get_typ(ident_id);
                self.typ = Some(id_type.clone());
                Ok(())
            }
            stream::Kind::ComponentApplication {
                called_comp_id,
                ref mut inputs,
                ..
            } => {
                // type all inputs and check their types
                inputs.iter_mut().try_for_each(|(id, input)| {
                    input.typ_check(symbols, errors)?;

                    let input_type = input.typ.as_ref().unwrap();
                    let expected_type = symbols.get_typ(*id);
                    input_type.expect(self.loc, expected_type).dewrap(errors)
                })?;

                // get the called ident type
                let comp_application_type = {
                    let mut outputs_types = symbols
                        .get_comp_outputs(called_comp_id)
                        .iter()
                        .map(|(_, output_ident)| symbols.get_typ(*output_ident).clone())
                        .collect::<Vec<_>>();
                    if outputs_types.len() == 1 {
                        outputs_types.pop().unwrap()
                    } else {
                        Typ::tuple(outputs_types)
                    }
                };

                self.typ = Some(comp_application_type);
                Ok(())
            }
            stream::Kind::Expression { ref mut expr } => {
                self.typ = Some(expr.typ_check(self.loc, symbols, errors)?);
                Ok(())
            }
            stream::Kind::RisingEdge { ref mut expr } => {
                expr.typ_check(symbols, errors)?;
                // check expr is a boolean
                let expr_type = expr.get_typ().unwrap().clone();
                let expected = Typ::bool();
                expr_type.expect(self.loc, &expected).dewrap(errors)?;
                // set the type
                self.typ = Some(expected);
                Ok(())
            }
            stream::Kind::WhenExpr {
                ref mut def_eq_pat,
                ref mut arms,
                ..
            } => {
                arms.iter_mut().try_for_each(|(pattern, guard, body)| {
                    // type pattern
                    pattern.typ_check(symbols, errors)?;
                    // type guard
                    guard.as_mut().map_or(Ok(()), |expr| {
                        expr.typ_check(symbols, errors)?;
                        expr.get_typ().unwrap().expect_bool(self.loc).dewrap(errors)
                    })?;
                    // type all equations
                    body.iter_mut()
                        .try_for_each(|stmt| stmt.typ_check(symbols, errors))?;
                    Ok(())
                })?;

                def_eq_pat.typ_check(symbols, errors)?;
                let pat_typ = def_eq_pat
                    .get_type()
                    .expect("internal error: should be typed by the previous statement");
                self.typ = Some(pat_typ.clone());

                Ok(())
            }
            stream::Kind::SomeEvent { ref mut expr } => {
                expr.typ_check(symbols, errors)?;
                let expr_type = expr.get_typ().unwrap().clone();
                // set the type
                self.typ = Some(Typ::option(expr_type));
                Ok(())
            }
            stream::Kind::NoneEvent => unreachable!("typ_check in expressions"),
        }
    }

    fn get_typ(&self) -> Option<&Typ> {
        self.typ.as_ref()
    }

    fn get_typ_mut(&mut self) -> Option<&mut Typ> {
        self.typ.as_mut()
    }
}

impl<E: Typing> Typing for Stmt<E> {
    // post-condition: expression associated with statement is typed and checked
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        self.pattern.typ_check(symbols, errors)?;
        let expected_type = self.pattern.typ.as_ref().unwrap();

        self.expr.typ_check(symbols, errors)?;
        let expr_type = self.expr.get_typ().unwrap();

        expr_type.expect(self.loc, expected_type).dewrap(errors)?;

        Ok(())
    }
}

impl Typing for stream::InitStmt {
    // post-condition: expression associated with statement is typed and checked
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        self.pattern.typ_check(symbols, errors)?;
        let expected_type = self.pattern.typ.as_ref().unwrap();

        self.expr.typ_check(symbols, errors)?;
        let expr_type = self.expr.get_typ().unwrap();

        expr_type.expect(self.loc, expected_type).dewrap(errors)?;

        Ok(())
    }
}

impl Pattern {
    /// Tries to type the given construct.
    pub fn typ_check(
        &mut self,
        expected_type: &Typ,
        symbols: &mut Ctx,
        errors: &mut Vec<Error>,
    ) -> TRes<()> {
        use pattern::Kind;
        match self.kind {
            Kind::Constant { ref constant } => {
                let pattern_type = constant.get_typ();
                pattern_type
                    .expect(self.loc, expected_type)
                    .dewrap(errors)?;
                self.typing = Some(pattern_type);
                Ok(())
            }
            Kind::Identifier { id } => {
                symbols.set_type(id, expected_type.clone());
                self.typing = Some(expected_type.clone());
                Ok(())
            }
            Kind::Structure {
                ref id,
                ref mut fields,
            } => {
                fields
                    .iter_mut()
                    .map(|(id, optional_pattern)| {
                        let expected_type = symbols.get_typ(*id).clone();
                        if let Some(pattern) = optional_pattern {
                            pattern.typ_check(&expected_type, symbols, errors)?;
                            // check pattern type
                            let pattern_type = pattern.get_typ().unwrap();
                            pattern_type.expect(self.loc, &expected_type).dewrap(errors)
                        } else {
                            Ok(())
                        }
                    })
                    .collect::<Vec<TRes<()>>>()
                    .into_iter()
                    .collect::<TRes<()>>()?;
                self.typing = Some(Typ::structure_str(symbols.get_name(*id).clone(), *id));
                Ok(())
            }
            Kind::Enumeration { ref enum_id, .. } => {
                self.typing = Some(Typ::enumeration_str(
                    symbols.get_name(*enum_id).clone(),
                    *enum_id,
                ));
                Ok(())
            }
            Kind::Tuple { ref mut elements } => match expected_type {
                Typ::Tuple {
                    elements: types, ..
                } => {
                    if elements.len() != types.len() {
                        bad!(errors, @self.loc => ErrorKind::incompatible_tuple());
                    }
                    elements
                        .iter_mut()
                        .zip(types)
                        .map(|(pattern, expected_type)| {
                            pattern.typ_check(expected_type, symbols, errors)
                        })
                        .collect::<Vec<TRes<()>>>()
                        .into_iter()
                        .collect::<TRes<()>>()?;
                    let types = elements
                        .iter()
                        .map(|pattern| pattern.get_typ().unwrap().clone())
                        .collect();
                    self.typing = Some(Typ::tuple(types));
                    Ok(())
                }
                _ => bad!(errors, @self.loc => ErrorKind::expected_tuple_pat()),
            },
            Kind::Default(_) => {
                self.typing = Some(Typ::any());
                Ok(())
            }
            Kind::PresentEvent { .. } => unreachable!("typ_check in patterns"),
        }
    }
}

impl EventPattern {
    /// Tries to type the given construct.
    pub fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        use event_pattern::Kind;
        match self.kind {
            Kind::Tuple { ref mut elements } => {
                elements
                    .iter_mut()
                    .try_for_each(|pattern| pattern.typ_check(symbols, errors))?;
                let types = elements
                    .iter()
                    .map(|pattern| pattern.get_typ().unwrap().clone())
                    .collect();
                self.typing = Some(Typ::tuple(types));
                Ok(())
            }
            Kind::Let {
                ref mut pattern,
                ref mut event_id,
            } => match symbols.get_typ(*event_id) {
                Typ::Option { ty, .. } => {
                    let expected_type = ty.as_ref().clone();
                    pattern.typ_check(&expected_type, symbols, errors)?;
                    self.typing = Some(expected_type);
                    Ok(())
                }
                ty => {
                    Err(error!(@self.loc => ErrorKind::expected_event(ty.clone()))).dewrap(errors)
                }
            },
            Kind::RisingEdge(ref mut expr) => {
                expr.typ_check(symbols, errors)?;
                let ty = expr.get_typ().unwrap();
                ty.expect_bool(self.loc).dewrap(errors)?;
                self.typing = Some(ty.clone());
                Ok(())
            }
        }
    }
}

impl stmt::Pattern {
    /// Tries to construct the type of the given construct.
    pub fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        match self.kind {
            stmt::Kind::Identifier { id } => {
                let typing = symbols.get_typ(id);
                self.typ = Some(typing.clone());
                Ok(())
            }
            stmt::Kind::Typed { id, ref typ } => {
                let expected_type = &symbols.get_typ(id);
                let sym = symbols.resolve_symbol(self.loc, id).dewrap(errors)?;
                typ.expect(sym.loc(), expected_type).dewrap(errors)?;
                // symbols.set_type(id, typ.clone());
                self.typ = Some(typ.clone());
                Ok(())
            }
            stmt::Kind::Tuple { ref mut elements } => {
                let types = elements
                    .iter_mut()
                    .map(|pattern| {
                        pattern.typ_check(symbols, errors)?;
                        Ok(pattern.typ.as_ref().unwrap().clone())
                    })
                    .collect::<Vec<TRes<_>>>()
                    .into_iter()
                    .collect::<TRes<Vec<_>>>()?;

                self.typ = Some(Typ::tuple(types));
                Ok(())
            }
        }
    }
}

impl Typing for Expr {
    fn typ_check(&mut self, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        self.typing = Some(self.kind.typ_check(self.loc, symbols, errors)?);
        Ok(())
    }
    fn get_typ(&self) -> Option<&Typ> {
        self.typing.as_ref()
    }
    fn get_typ_mut(&mut self) -> Option<&mut Typ> {
        self.typing.as_mut()
    }
}

impl<E: Typing> expr::Kind<E> {
    /// Tries to type the given construct.
    fn typ_check(&mut self, loc: Loc, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<Typ> {
        let mut typing = ExprTyping::new(loc, symbols, errors);
        match self {
            expr::Kind::Constant { constant } => Ok(constant.get_typ()),
            expr::Kind::Identifier { id } => {
                let typing = symbols.get_typ(*id);
                Ok(typing.clone())
            }
            expr::Kind::UnOp { op, expr } => typing.unop(op, expr.as_mut()),
            expr::Kind::BinOp { op, lft, rgt } => typing.binop(op, lft.as_mut(), rgt.as_mut()),
            expr::Kind::IfThenElse { cnd, thn, els } => {
                typing.if_then_else(cnd.as_mut(), thn.as_mut(), els.as_mut())
            }
            expr::Kind::Application { fun: f, inputs } => typing.application(f.as_mut(), inputs),
            expr::Kind::Lambda { inputs, expr, .. } => typing.lambda(inputs, expr.as_mut()),
            expr::Kind::Structure { id, fields } => typing.structure(*id, fields),
            expr::Kind::Array { elements } => typing.array(elements),
            expr::Kind::Tuple { elements } => typing.tuple(elements),
            expr::Kind::MatchExpr { expr, arms } => typing.matching(expr.as_mut(), arms),
            expr::Kind::FieldAccess { expr, field } => typing.field_access(expr.as_mut(), field),
            expr::Kind::Map { expr, fun } => typing.map(expr.as_mut(), fun.as_mut()),
            expr::Kind::Fold { array, init, fun } => {
                typing.fold(array.as_mut(), init.as_mut(), fun.as_mut())
            }
            expr::Kind::Sort { expr, fun } => typing.sort(expr.as_mut(), fun.as_mut()),
            expr::Kind::Zip { arrays } => typing.zip(arrays),
            expr::Kind::TupleElementAccess {
                expr,
                element_number,
            } => typing.tuple_element_access(expr.as_mut(), *element_number),
            expr::Kind::ArrayAccess { expr, index } => typing.array_access(
                expr.as_mut(),
                index.base10_parse().map_err(|_| ErrorDetected)?,
            ),
            expr::Kind::Enumeration { enum_id, .. } => typing.enumeration(*enum_id),
        }
    }
}

struct ExprTyping<'a, E: Typing> {
    pub loc: Loc,
    table: &'a mut Ctx,
    errors: &'a mut Vec<Error>,
    _phantom: std::marker::PhantomData<E>,
}
impl<'a, E: Typing> ExprTyping<'a, E> {
    fn new(loc: Loc, table: &'a mut Ctx, errors: &'a mut Vec<Error>) -> Self {
        Self {
            loc,
            table,
            errors,
            _phantom: std::marker::PhantomData,
        }
    }

    fn lambda(&mut self, inputs: &[usize], expr: &mut Expr) -> TRes<Typ> {
        // type the abstracted expression with the local context
        expr.typ_check(self.table, self.errors)?;

        // compute lambda type
        let input_types = inputs
            .iter()
            .map(|id| self.table.get_typ(*id).clone())
            .collect::<Vec<_>>();
        let lambda_type = Typ::function(input_types, expr.get_typ().unwrap().clone());

        Ok(lambda_type)
    }

    fn application(&mut self, f: &mut E, inputs: &mut [E]) -> TRes<Typ> {
        // type all inputs
        for input in inputs.iter_mut() {
            input.typ_check(self.table, self.errors)?;
        }

        let input_types = inputs
            .iter()
            .map(|input| input.get_typ().unwrap().clone())
            .collect::<Vec<_>>();

        // type the function expression
        f.typ_check(self.table, self.errors)?;

        // compute the application type
        let application_type =
            f.get_typ_mut()
                .unwrap()
                .apply(input_types, self.loc, self.errors)?;

        Ok(application_type)
    }

    fn array(&mut self, elms: &mut [E]) -> TRes<Typ> {
        if elms.is_empty() {
            bad!(self.errors, @self.loc => ErrorKind::expected_input());
        }

        elms.iter_mut()
            .try_for_each(|element| element.typ_check(self.table, self.errors))?;

        let first_type = elms[0]
            .get_typ()
            .expect("internal error: arrays cannot be empty");
        elms.iter().try_for_each(|element| {
            let element_type = element.get_typ().unwrap();
            element_type
                .expect(self.loc, first_type)
                .dewrap(self.errors)
        })?;

        let array_type = Typ::array(first_type.clone(), elms.len());

        Ok(array_type)
    }

    fn binop(&mut self, op: &BOp, lft: &mut E, rgt: &mut E) -> TRes<Typ> {
        // get expressions type
        lft.typ_check(self.table, self.errors)?;
        let lft_type = lft.get_typ().unwrap().clone();
        rgt.typ_check(self.table, self.errors)?;
        let rgt_type = rgt.get_typ().unwrap().clone();

        // get binop type
        let mut binop_type = op.get_typ();

        binop_type.apply(vec![lft_type, rgt_type], self.loc, self.errors)
    }

    fn enumeration(&mut self, enum_id: usize) -> TRes<Typ> {
        Ok(Typ::Enumeration {
            name: self.table.get_name(enum_id).clone(),
            id: enum_id,
        })
    }

    fn field_access(&mut self, expr: &mut E, field: &Ident) -> TRes<Typ> {
        expr.typ_check(self.table, self.errors)?;

        match expr.get_typ().unwrap() {
            Typ::Structure { name, id } => {
                let symbol = self
                    .table
                    .get_symbol(*id)
                    .expect("internal error: there should be a symbol");
                match symbol.kind() {
                    ir0::symbol::SymbolKind::Structure { fields } => {
                        let option_field_type = fields
                            .iter()
                            .filter(|id| {
                                let field_name = self.table.get_name(**id);
                                field == field_name
                            })
                            .map(|id| self.table.get_typ(*id).clone())
                            .next();
                        if let Some(field_type) = option_field_type {
                            Ok(field_type)
                        } else {
                            bad!(self.errors, @field.loc() =>
                                ErrorKind::unknown_field(name.to_string(), field.to_string())
                            )
                        }
                    }
                    _ => noErrorDesc!(),
                }
            }
            given_type => {
                bad!(self.errors, @self.loc => ErrorKind::expected_structure(given_type.clone()))
            }
        }
    }

    fn fold(&mut self, expr: &mut E, init: &mut E, fun: &mut E) -> TRes<Typ> {
        // type the expression
        expr.typ_check(self.table, self.errors)?;

        // verify it is an array
        match expr.get_typ().unwrap() {
            Typ::Array {
                ty: element_type, ..
            } => {
                // type the initialization expression
                init.typ_check(self.table, self.errors)?;
                let initialization_type = init.get_typ().unwrap();

                // type the function expression
                fun.typ_check(self.table, self.errors)?;
                let function_type = fun.get_typ_mut().unwrap();

                // apply the function type to the type of the initialization and array's elements
                let new_type = function_type.apply(
                    vec![initialization_type.clone(), *element_type.clone()],
                    self.loc,
                    self.errors,
                )?;

                // check the new type is equal to the initialization type
                new_type
                    .expect(self.loc, initialization_type)
                    .dewrap(self.errors)?;

                Ok(new_type)
            }
            given_type => {
                bad!(self.errors, @self.loc => ErrorKind::expected_array(given_type.clone()))
            }
        }
    }

    fn if_then_else(&mut self, cnd: &mut E, thn: &mut E, els: &mut E) -> TRes<Typ> {
        // get expressions type
        cnd.typ_check(self.table, self.errors)?;
        let cnd_type = cnd.get_typ().unwrap().clone();
        thn.typ_check(self.table, self.errors)?;
        let thn_type = thn.get_typ().unwrap().clone();
        els.typ_check(self.table, self.errors)?;
        let els_type = els.get_typ().unwrap().clone();

        // get if_then_else type
        let mut if_then_else_type = OtherOp::IfThenElse.get_typ();

        if_then_else_type.apply(vec![cnd_type, thn_type, els_type], self.loc, self.errors)
    }

    fn map(&mut self, expr: &mut E, fun: &mut E) -> TRes<Typ> {
        // type the expression
        expr.typ_check(self.table, self.errors)?;

        // verify it is an array
        match expr.get_typ().unwrap() {
            Typ::Array {
                ty: element_type,
                size,
                bracket_token,
                semi_token,
            } => {
                // type the function expression
                fun.typ_check(self.table, self.errors)?;
                let function_type = fun.get_typ_mut().unwrap();

                // apply the function type to the type of array's elements
                let new_element_type =
                    function_type.apply(vec![*element_type.clone()], self.loc, self.errors)?;

                Ok(Typ::Array {
                    ty: new_element_type.into(),
                    size: size.clone(),
                    bracket_token: *bracket_token,
                    semi_token: *semi_token,
                })
            }
            given_type => {
                bad!(self.errors, @self.loc => ErrorKind::expected_array(given_type.clone()))
            }
        }
    }

    fn matching(&mut self, expr: &mut E, arms: &mut [ir1::expr::Arm<E>]) -> TRes<Typ> {
        expr.typ_check(self.table, self.errors)?;

        let expr_type = expr.get_typ().unwrap();

        arms.iter_mut().try_for_each(
            |(pattern, optional_test_expression, body, arm_expression)| {
                // check it matches pattern type
                pattern.typ_check(expr_type, self.table, self.errors)?;

                optional_test_expression
                    .as_mut()
                    .map_or(Ok(()), |expression| {
                        expression.typ_check(self.table, self.errors)?;
                        expression
                            .get_typ()
                            .unwrap()
                            .expect_bool(self.loc)
                            .dewrap(self.errors)
                    })?;

                // set types for every pattern
                body.iter_mut().try_for_each(|statement| {
                    statement.pattern.typ_check(self.table, self.errors)
                })?;

                // type all equations
                body.iter_mut()
                    .try_for_each(|statement| statement.typ_check(self.table, self.errors))?;

                arm_expression.typ_check(self.table, self.errors)
            },
        )?;

        let first_type = arms[0].3.get_typ().unwrap();
        arms.iter().try_for_each(|(_, _, _, arm_expression)| {
            let arm_expression_type = arm_expression.get_typ().unwrap();
            arm_expression_type
                .expect(self.loc, first_type)
                .dewrap(self.errors)
        })?;

        Ok(first_type.clone())
    }

    fn sort(&mut self, expr: &mut E, fun: &mut E) -> TRes<Typ> {
        // type the expression
        expr.typ_check(self.table, self.errors)?;

        // verify it is an array
        match expr.get_typ().unwrap() {
            Typ::Array {
                ty: element_type,
                size,
                bracket_token,
                semi_token,
            } => {
                // type the function expression
                fun.typ_check(self.table, self.errors)?;
                let function_type = fun.get_typ_mut().unwrap();

                // check it is a sorting function: (element_type, element_type) -> int
                function_type
                    .expect(
                        self.loc,
                        &Typ::function(
                            vec![*element_type.clone(), *element_type.clone()],
                            Typ::int(),
                        ),
                    )
                    .dewrap(self.errors)?;

                Ok(Typ::Array {
                    ty: element_type.clone(),
                    size: size.clone(),
                    bracket_token: *bracket_token,
                    semi_token: *semi_token,
                })
            }
            given_type => {
                bad!(self.errors, @self.loc => ErrorKind::expected_array(given_type.clone()))
            }
        }
    }

    fn structure(&mut self, id: usize, fields: &mut [(usize, E)]) -> TRes<Typ> {
        // type each field and check their type
        fields.iter_mut().try_for_each(|(id, expression)| {
            expression.typ_check(self.table, self.errors)?;
            let expression_type = expression.get_typ().unwrap();
            let expected_type = self.table.get_typ(*id);
            expression_type
                .expect(self.loc, expected_type)
                .dewrap(self.errors)
        })?;

        Ok(Typ::Structure {
            name: self.table.get_name(id).clone(),
            id,
        })
    }

    fn tuple_element_access(&mut self, expr: &mut E, elm: usize) -> TRes<Typ> {
        expr.typ_check(self.table, self.errors)?;

        match expr.get_typ().unwrap() {
            Typ::Tuple { elements, .. } => {
                let option_element_type = elements.iter().nth(elm);
                if let Some(element_type) = option_element_type {
                    Ok(element_type.clone())
                } else {
                    bad!(self.errors, @self.loc => ErrorKind::oob())
                }
            }
            given_type => {
                bad!(self.errors, @self.loc => ErrorKind::expected_tuple(given_type.clone()))
            }
        }
    }

    fn array_access(&mut self, expr: &mut E, index: usize) -> TRes<Typ> {
        expr.typ_check(self.table, self.errors)?;

        match expr.get_typ().unwrap() {
            Typ::Array { ty, size, .. } => {
                let size: usize = size.base10_parse().map_err(|_| ErrorDetected)?;
                if size <= index {
                    bad!(self.errors, @self.loc => ErrorKind::oob());
                }
                Ok((**ty).clone())
            }
            given_type => {
                bad!(self.errors, @self.loc => ErrorKind::expected_tuple(given_type.clone()))
            }
        }
    }

    fn tuple(&mut self, elms: &mut [E]) -> TRes<Typ> {
        // debug_assert!(elms.len() >= 1);
        // the `when` item might create empty tuples in case there are only rising edges

        let elms_types = elms
            .iter_mut()
            .map(|element| {
                element.typ_check(self.table, self.errors)?;
                Ok(element
                    .get_typ()
                    .expect("internal error: should be typed")
                    .clone())
            })
            .collect::<TRes<Vec<Typ>>>()?;

        let tuple_type = Typ::tuple(elms_types);

        Ok(tuple_type)
    }

    fn unop(&mut self, op: &UOp, expr: &mut E) -> TRes<Typ> {
        // get expression type
        expr.typ_check(self.table, self.errors)?;
        let expr_type = expr.get_typ().unwrap().clone();

        // get unop type
        let mut unop_type = op.get_typ();

        unop_type.apply(vec![expr_type], self.loc, self.errors)
    }

    fn zip(&mut self, arrays: &mut [E]) -> TRes<Typ> {
        if arrays.is_empty() {
            bad!(self.errors, @self.loc => ErrorKind::expected_input());
        }

        arrays
            .iter_mut()
            .try_for_each(|array| array.typ_check(self.table, self.errors))?;

        let length = (match arrays[0].get_typ().unwrap() {
            Typ::Array { size: n, .. } => Ok(n),
            ty => bad!(self.errors, @self.loc => ErrorKind::expected_array(ty.clone())),
        })?;
        let tuple_types = arrays
            .iter()
            .map(|array| match array.get_typ().unwrap() {
                Typ::Array { ty, size: n, .. } if n == length => Ok(*ty.clone()),
                Typ::Array { size: n, .. } => {
                    bad!(self.errors, @self.loc => ErrorKind::incompatible_length(
                        n.base10_parse().unwrap(),
                        length.base10_parse().unwrap(),
                    ))
                }
                ty => {
                    bad!(self.errors, @self.loc => ErrorKind::expected_array(ty.clone()))
                }
            })
            .collect::<TRes<Vec<Typ>>>()?;

        let array_type = if tuple_types.len() > 1 {
            Typ::array(Typ::tuple(tuple_types), length.base10_parse().unwrap())
        } else {
            Typ::array(
                tuple_types.first().unwrap().clone(),
                length.base10_parse().unwrap(),
            )
        };

        Ok(array_type)
    }
}
