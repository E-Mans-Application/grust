prelude! {}

mod interface;
mod isles;
mod trigger;

pub use self::{isles::*, trigger::*};

/// Turns an [ir1] element in a [ir2] element, implemented for [ir1] types.
pub trait Ir1IntoIr2<Ctx> {
    /// [ir2] type constructed.
    type Ir2;

    /// [ir1] to [ir2].
    fn into_ir2(self, ctx: Ctx) -> Self::Ir2;
    /// Option type information.
    fn try_get_typ(&self) -> Option<&Typ> {
        None
    }
    /// True if the [ir2] element is an if-then-else operator.
    fn is_if_then_else(&self, _ctx: Ctx) -> bool {
        false
    }
}

impl Ir1IntoIr2<&'_ ir0::Ctx> for ir1::Component {
    type Ir2 = Option<StateMachine>;

    fn into_ir2(self, ctx: &ir0::Ctx) -> Self::Ir2 {
        match self.body_or_path {
            Either::Left(body) => {
                // get component name
                let name = ctx.get_name(self.sign.id);

                // get component inputs
                let inputs = ctx
                    .get_comp_inputs(self.sign.id)
                    .iter()
                    .map(|id| (ctx.get_name(*id).clone(), ctx.get_typ(*id).clone()));

                // get component outputs
                let outputs = ctx.get_comp_outputs(self.sign.id);

                // get memory/state elements
                let (elements, state_elements_init, state_elements_step) =
                    memory_state_elements(body.memory, ctx);

                // transform contract
                let contract = body.contract.into_ir2(ctx);
                let invariant_init = contract.invariant.clone();

                use state_machine::*;

                // 'init' method
                let init = Init::new(name.clone(), state_elements_init, invariant_init);

                // 'step' method
                let step = {
                    // logs
                    let logs = body.logs.into_iter().map(|id| {
                        let scope = ctx.get_scope(id);
                        let ident = ctx.get_name(id).clone();
                        let expr = match scope {
                            Scope::Input => Expr::input_access(ident.clone()),
                            Scope::Output | Scope::Local => Expr::ident(ident.clone()),
                            Scope::VeryLocal => noErrorDesc!(),
                        };
                        Stmt::log(ident, expr)
                    });
                    // body stmts
                    let body = match para::Stmts::of_ir1(&body.statements, ctx, &body.graph) {
                        Ok(stmts) => stmts,
                        Err(e) => panic!(
                            "failed to generate (step) synced body of component `{}`:\n{}",
                            name, e
                        ),
                    };
                    Step::new(
                        name.clone(),
                        body,
                        state_elements_step,
                        logs,
                        outputs.iter().map(|(_, id)| ctx.get_name(*id).clone()),
                        contract,
                    )
                };

                // 'input' structure
                let input = Input {
                    comp_name: name.clone(),
                    elements: inputs.collect(),
                };
                let output = Output {
                    comp_name: name.clone(),
                    elements: outputs
                        .iter()
                        .map(|(_, id)| (ctx.get_name(*id).clone(), ctx.get_typ(*id).clone()))
                        .collect(),
                };

                // 'state' structure
                let state = State {
                    comp_name: name.clone(),
                    elements,
                    step,
                    init,
                };

                Some(StateMachine::new(name.clone(), input, output, state))
            }
            Either::Right(_) => None,
        }
    }
}

/// Get state elements from memory.
pub fn memory_state_elements(
    mem: ir1::Memory,
    ctx: &ir0::Ctx,
) -> (
    Vec<state_machine::StateElmInfo>,
    Vec<state_machine::StateElmInit>,
    Vec<state_machine::StateElmStep>,
) {
    use ir1::memory::*;
    use itertools::Itertools;
    use state_machine::{StateElmInfo, StateElmInit, StateElmStep};

    let (mut elements, mut inits, mut steps) = (vec![], vec![], vec![]);
    for (
        _,
        Buffer {
            ident,
            typing,
            init,
            id,
            ..
        },
    ) in mem.buffers.into_iter().sorted_by_key(|(id, _)| id.clone())
    {
        let scope = ctx.get_scope(id);
        let mem_ident = ident.to_last_var();
        elements.push(StateElmInfo::buffer(mem_ident.clone(), typing));
        inits.push(StateElmInit::buffer(mem_ident.clone(), init.into_ir2(ctx)));
        steps.push(StateElmStep::new(
            mem_ident,
            match scope {
                Scope::Input => Expr::input_access(ident),
                Scope::Output | Scope::Local => Expr::ident(ident),
                Scope::VeryLocal => noErrorDesc!(),
            },
        ))
    }
    mem.called_comps
        .into_iter()
        .sorted_by_key(|(id, _)| *id)
        .for_each(|(memory_id, CalledComponent { comp_id, .. })| {
            let memory_name = ctx.get_name(memory_id);
            let comp_name = ctx.get_name(comp_id);
            let path_opt = ctx.try_get_comp_path(comp_id);
            elements.push(StateElmInfo::called_comp(
                memory_name.clone(),
                comp_name.clone(),
                path_opt.cloned(),
            ));
            inits.push(StateElmInit::called_comp(
                memory_name.clone(),
                comp_name.clone(),
                path_opt.cloned(),
            ));
        });
    mem.ghost_comps
        .into_iter()
        .sorted_by_key(|(id, _)| *id)
        .for_each(|(memory_id, GhostComponent { comp_id, .. })| {
            let memory_name = ctx.get_name(memory_id);
            let comp_name = ctx.get_name(comp_id);
            let path_opt = ctx.try_get_comp_path(comp_id);
            elements.push(StateElmInfo::called_comp(
                memory_name.clone(),
                comp_name.clone(),
                path_opt.cloned(),
            ));
            inits.push(StateElmInit::called_comp(
                memory_name.clone(),
                comp_name.clone(),
                path_opt.cloned(),
            ));
        });

    (elements, inits, steps)
}

impl Ir1IntoIr2<&'_ ir0::Ctx> for ir1::Contract {
    type Ir2 = Contract;

    fn into_ir2(self, ctx: &ir0::Ctx) -> Self::Ir2 {
        Contract {
            requires: self
                .requires
                .into_iter()
                .map(|term| term.into_ir2(ctx))
                .collect(),
            ensures: self
                .ensures
                .into_iter()
                .map(|term| term.into_ir2(ctx))
                .collect(),
            invariant: self
                .invariant
                .into_iter()
                .map(|term| term.into_ir2(ctx))
                .collect(),
        }
    }
}

mod term {
    prelude! {
        ir1::contract::{Term, Kind},
    }

    impl Ir1IntoIr2<&'_ ir0::Ctx> for Term {
        type Ir2 = contract::Term;

        fn into_ir2(self, ctx: &ir0::Ctx) -> Self::Ir2 {
            match self.kind {
                Kind::Constant { constant } => contract::Term::literal(constant),
                Kind::Paren { term } => contract::Term::paren(term.into_ir2(ctx)),
                Kind::Identifier { id } => {
                    let name = ctx.get_name(id);
                    let views = ctx.get_typ(id).needs_view();
                    match ctx.get_scope(id) {
                        Scope::Input => contract::Term::input(name.clone(), views),
                        Scope::Output => contract::Term::output(name.clone(), views),
                        Scope::Local => contract::Term::ident(name.clone(), views),
                        Scope::VeryLocal => noErrorDesc!("you should not do that with this ident"),
                    }
                }
                Kind::Last { ident_id, .. } => {
                    let name = ctx.get_name(ident_id).clone();
                    let views = ctx.get_typ(ident_id).needs_view();
                    contract::Term::mem(name, views)
                }
                Kind::Enumeration {
                    enum_id,
                    element_id,
                } => contract::Term::enumeration(
                    ctx.get_name(enum_id).clone(),
                    ctx.get_name(element_id).clone(),
                    None,
                ),
                Kind::Unary { op, term } => contract::Term::unop(op, term.into_ir2(ctx)),
                Kind::Binary { op, left, right } => {
                    contract::Term::binop(op, left.into_ir2(ctx), right.into_ir2(ctx))
                }
                Kind::ForAll { id, term } => {
                    let name = ctx.get_name(id);
                    let ty = ctx.get_typ(id).clone();
                    let term = term.into_ir2(ctx);
                    contract::Term::forall(name.clone(), ty, term)
                }
                Kind::Implication { left, right } => {
                    contract::Term::implication(left.into_ir2(ctx), right.into_ir2(ctx))
                }
                Kind::PresentEvent { event_id, pattern } => match ctx.get_typ(event_id) {
                    Typ::Option { .. } => {
                        let name = ctx.get_name(pattern).clone();
                        contract::Term::some(contract::Term::ident(name, false))
                    }
                    _ => noErrorDesc!(),
                },
                Kind::Application { fun_id, inputs, .. } => {
                    let function = ctx.get_name(fun_id).clone();
                    let arguments = inputs
                        .into_iter()
                        .map(|input| input.into_ir2(ctx))
                        .collect::<Vec<_>>();
                    contract::Term::fun_call(function, arguments)
                }
                Kind::ComponentCall {
                    memory_id,
                    comp_id,
                    inputs,
                } => {
                    let memory_ident = ctx
                        .get_name(memory_id.expect(
                            "internal error: should be defined in `ir1::contract::Term::memorize`",
                        ))
                        .clone();
                    let comp_name = ctx.get_name(comp_id).clone();
                    let input_fields = inputs
                        .into_iter()
                        .map(|(id, term)| (ctx.get_name(id).clone(), term.into_ir2(ctx)))
                        .collect_vec();
                    let input_ty = comp_name.to_input_ty();
                    contract::Term::comp_call(memory_ident, comp_name, input_ty, input_fields)
                }
            }
        }
    }
}

impl<'a, E> Ir1IntoIr2<&'a ir0::Ctx> for ir1::expr::Kind<E>
where
    E: Ir1IntoIr2<&'a ir0::Ctx, Ir2 = Expr>,
{
    type Ir2 = Expr;

    fn into_ir2(self, ctx: &'a ir0::Ctx) -> Self::Ir2 {
        match self {
            Self::Constant { constant, .. } => Expr::Literal { literal: constant },
            Self::Identifier { id, .. } => {
                let name = ctx.get_name(id).clone();
                if ctx.is_function(id) {
                    if let Some(path) = ctx.try_get_function_path(id) {
                        Expr::Path { path: path.clone() }
                    } else {
                        Expr::Identifier { identifier: name }
                    }
                } else {
                    let scope = ctx.get_scope(id);
                    match scope {
                        Scope::Input => Expr::InputAccess { identifier: name },
                        Scope::Output | Scope::Local | Scope::VeryLocal => {
                            Expr::Identifier { identifier: name }
                        }
                    }
                }
            }
            Self::UnOp { op, expr } => {
                let expr = expr.into_ir2(ctx);
                Expr::unop(op, expr)
            }
            Self::BinOp { op, lft, rgt } => {
                let lft = lft.into_ir2(ctx);
                let rgt = rgt.into_ir2(ctx);
                Expr::binop(op, lft, rgt)
            }
            Self::IfThenElse { cnd, thn, els } => {
                let cnd = cnd.into_ir2(ctx);
                let thn = thn.into_ir2(ctx);
                let els = els.into_ir2(ctx);
                Expr::ite(
                    cnd,
                    Block::new(vec![Stmt::ExprLast { expr: thn }]),
                    Block::new(vec![Stmt::ExprLast { expr: els }]),
                )
            }
            Self::Application { fun, inputs, .. } => {
                let arguments = inputs
                    .into_iter()
                    .map(|input| input.into_ir2(ctx))
                    .collect();
                Expr::FunctionCall {
                    function: Box::new(fun.into_ir2(ctx)),
                    arguments,
                }
            }
            Self::Lambda { inputs, expr, .. } => {
                let inputs = inputs
                    .iter()
                    .map(|id| (ctx.get_name(*id).clone(), ctx.get_typ(*id).clone()))
                    .collect();
                let output = expr
                    .try_get_typ()
                    .expect("internal error: it should be typed")
                    .clone();
                let body = Expr::block(Block::new(vec![Stmt::expr_last(expr.into_ir2(ctx))]));
                Expr::lambda(false, inputs, output, body)
            }
            Self::Structure { id, fields, .. } => Expr::Structure {
                name: ctx.get_name(id).clone(),
                fields: fields
                    .into_iter()
                    .map(|(id, expr)| (ctx.get_name(id).clone(), expr.into_ir2(ctx)))
                    .collect(),
            },
            Self::Enumeration { enum_id, elem_id } => Expr::Enumeration {
                name: ctx.get_name(enum_id).clone(),
                element: ctx.get_name(elem_id).clone(),
            },
            Self::Array { elements } => Expr::Array {
                elements: elements
                    .into_iter()
                    .map(|element| element.into_ir2(ctx))
                    .collect(),
            },
            Self::Tuple { elements } => Expr::Tuple {
                elements: elements
                    .into_iter()
                    .map(|element| element.into_ir2(ctx))
                    .collect(),
            },
            Self::MatchExpr { expr, arms, .. } => Expr::MatchExpr {
                matched: Box::new(expr.into_ir2(ctx)),
                arms: arms
                    .into_iter()
                    .map(|(pattern, guard, body, expr)| {
                        (
                            pattern.into_ir2(ctx),
                            guard.map(|expr| expr.into_ir2(ctx)),
                            if body.is_empty() {
                                expr.into_ir2(ctx)
                            } else {
                                let mut statements = body
                                    .into_iter()
                                    .map(|statement| statement.into_ir2(ctx))
                                    .collect_vec();
                                statements.push(Stmt::ExprLast {
                                    expr: expr.into_ir2(ctx),
                                });
                                Expr::Block {
                                    block: Block { statements },
                                }
                            },
                        )
                    })
                    .collect(),
            },
            Self::FieldAccess { expr, field, .. } => Expr::FieldAccess {
                expr: Box::new(expr.into_ir2(ctx)),
                field: FieldIdentifier::Named(field),
            },
            Self::TupleElementAccess {
                expr,
                element_number,
                ..
            } => Expr::FieldAccess {
                expr: Box::new(expr.into_ir2(ctx)),
                field: FieldIdentifier::Unnamed(element_number),
            },
            Self::ArrayAccess { expr, index, .. } => Expr::ArrayAccess {
                expr: Box::new(expr.into_ir2(ctx)),
                index,
            },
            Self::Map { expr, fun, .. } => Expr::Map {
                mapped: Box::new(expr.into_ir2(ctx)),
                function: Box::new(fun.into_ir2(ctx)),
            },
            Self::Fold {
                array, init, fun, ..
            } => Expr::fold(array.into_ir2(ctx), init.into_ir2(ctx), fun.into_ir2(ctx)),
            Self::Sort { expr, fun, .. } => Expr::Sort {
                sorted: Box::new(expr.into_ir2(ctx)),
                function: Box::new(fun.into_ir2(ctx)),
            },
            Self::Zip { arrays, .. } => Expr::Zip {
                arrays: arrays
                    .into_iter()
                    .map(|element| element.into_ir2(ctx))
                    .collect(),
            },
        }
    }

    fn is_if_then_else(&self, ctx: &ir0::Ctx) -> bool {
        match self {
            Self::Identifier { id, .. } => OtherOp::IfThenElse
                .to_string()
                .eq(&ctx.get_name(*id).to_string()),
            _ => false,
        }
    }
}

impl Ir1IntoIr2<&'_ ir0::Ctx> for ir1::Expr {
    type Ir2 = Expr;

    fn into_ir2(self, ctx: &ir0::Ctx) -> Self::Ir2 {
        self.kind.into_ir2(ctx)
    }
    fn try_get_typ(&self) -> Option<&Typ> {
        self.typing.as_ref()
    }

    fn is_if_then_else(&self, ctx: &ir0::Ctx) -> bool {
        self.kind.is_if_then_else(ctx)
    }
}

impl Ir1IntoIr2<&'_ mut ir0::Ctx> for ir1::File {
    type Ir2 = Project;

    fn into_ir2(self, mut ctx: &mut ir0::Ctx) -> Project {
        let mut items = vec![];

        let typedefs = self
            .typedefs
            .into_iter()
            .map(|typedef| typedef.into_ir2(ctx));
        items.extend(typedefs);

        let functions = self
            .functions
            .into_iter()
            .filter_map(|function| function.into_ir2(ctx))
            .map(Item::Function);
        items.extend(functions);

        let state_machines = self
            .components
            .into_iter()
            .filter_map(|component| component.into_ir2(ctx))
            .map(Item::StateMachine);
        items.extend(state_machines);

        if !self.interface.services.is_empty() {
            let execution_machine = self.interface.into_ir2(&mut ctx);
            items.push(Item::ExecutionMachine(execution_machine));
        }

        Project { items }
    }
}

impl Ir1IntoIr2<&'_ ir0::Ctx> for ir1::Function {
    type Ir2 = Option<Function>;

    fn into_ir2(self, ctx: &ir0::Ctx) -> Self::Ir2 {
        // get function name
        let name = ctx.get_name(self.id).clone();

        // get function inputs
        let inputs = ctx
            .get_function_input(self.id)
            .iter()
            .map(|id| (ctx.get_name(*id).clone(), ctx.get_typ(*id).clone()))
            .collect_vec();

        // get function output type
        let output = ctx.get_function_output_type(self.id).clone();

        match self.body_or_path {
            Either::Left(body) => {
                // Transforms into [ir2] statements
                let mut statements = body
                    .statements
                    .into_iter()
                    .map(|statement| statement.into_ir2(ctx))
                    .collect_vec();

                // Logs
                let logs = body.logs.into_iter().map(|id| {
                    let scope = ctx.get_scope(id);
                    let ident = ctx.get_name(id).clone();
                    let expr = match scope {
                        Scope::Input => Expr::input_access(ident.clone()),
                        Scope::Output | Scope::Local => Expr::ident(ident.clone()),
                        Scope::VeryLocal => noErrorDesc!(),
                    };
                    Stmt::log(ident, expr)
                });
                statements.extend(logs);

                // return stmt
                statements.push(Stmt::ExprLast {
                    expr: body.returned.into_ir2(ctx),
                });

                // transform contract
                let contract = body.contract.into_ir2(ctx);

                // Body
                let body = Block { statements };

                Some(Function::new(name, inputs, output, body, contract))
            }
            Either::Right(_) => None,
        }
    }
}

impl Ir1IntoIr2<&'_ ir0::Ctx> for ir1::Pattern {
    type Ir2 = Pattern;

    fn into_ir2(self, ctx: &ir0::Ctx) -> Self::Ir2 {
        use ir1::pattern::Kind;
        match self.kind {
            Kind::Identifier { id } => Pattern::Identifier {
                name: ctx.get_name(id).clone(),
            },
            Kind::Constant { constant } => Pattern::Literal { literal: constant },
            Kind::Structure { id, fields } => Pattern::Structure {
                path: ctx.get_name(id).clone().into(),
                fields: fields
                    .into_iter()
                    .map(|(id, optional_pattern)| {
                        (
                            ctx.get_name(id).clone(),
                            optional_pattern.map_or(
                                Pattern::Identifier {
                                    name: ctx.get_name(id).clone(),
                                },
                                |pattern| pattern.into_ir2(ctx),
                            ),
                        )
                    })
                    .collect(),
            },
            Kind::Enumeration { enum_id, elem_id } => Pattern::Enumeration {
                enum_name: ctx.get_name(enum_id).clone(),
                elem_name: ctx.get_name(elem_id).clone(),
                element: None,
            },
            Kind::Tuple { elements } => Pattern::Tuple {
                elements: elements
                    .into_iter()
                    .map(|element| element.into_ir2(ctx))
                    .collect(),
            },
            Kind::Default(loc) => Pattern::Default(loc),
            Kind::PresentEvent { event_id, pattern } => match ctx.get_typ(event_id) {
                Typ::Option { .. } => Pattern::some(pattern.into_ir2(ctx)),
                _ => noErrorDesc!(),
            },
        }
    }
}

impl Ir1IntoIr2<&'_ ir0::Ctx> for ir1::stmt::Pattern {
    type Ir2 = Pattern;

    fn into_ir2(self, ctx: &ir0::Ctx) -> Self::Ir2 {
        match self.kind {
            ir1::stmt::Kind::Identifier { id } | ir1::stmt::Kind::Typed { id, .. } => {
                Pattern::Identifier {
                    name: ctx.get_name(id).clone(),
                }
            }
            ir1::stmt::Kind::Tuple { elements } => Pattern::Tuple {
                elements: elements
                    .into_iter()
                    .map(|element| element.into_ir2(ctx))
                    .collect(),
            },
        }
    }
}

impl Pattern {
    fn comp_output(pat: ir1::stmt::Pattern, comp_id: usize, ctx: &ir0::Ctx) -> Self {
        let mut outputs = ctx.get_comp_outputs(comp_id).clone();
        let output_ty = ctx.get_name(comp_id).to_output_ty();
        let path = if let Some(path) = ctx.try_get_comp_path(comp_id) {
            let mut path = path.clone();
            path.segments.pop();
            let mut output_path = path;
            output_path.segments.push(output_ty.into());
            output_path
        } else {
            output_ty.into()
        };
        match pat.kind {
            ir1::stmt::Kind::Identifier { id } | ir1::stmt::Kind::Typed { id, .. } => {
                let (_, out_id) = outputs
                    .pop()
                    .expect("internal error: type should be already checked");
                let out_name = ctx.get_name(out_id).clone();
                debug_assert!(
                    outputs.is_empty(),
                    "internal error: type should be already checked"
                );
                Pattern::Structure {
                    path,
                    fields: vec![(
                        out_name,
                        Pattern::Identifier {
                            name: ctx.get_name(id).clone(),
                        },
                    )],
                }
            }
            ir1::stmt::Kind::Tuple { elements } => {
                debug_assert!(
                    outputs.len() == elements.len(),
                    "internal error: type should be already checked"
                );

                Pattern::Structure {
                    path,
                    fields: elements
                        .into_iter()
                        .zip(outputs)
                        .map(|(pat, (_, out_id))| (ctx.get_name(out_id).clone(), pat.into_ir2(ctx)))
                        .collect(),
                }
            }
        }
    }
}

impl<'a, E> Ir1IntoIr2<&'a ir0::Ctx> for ir1::Stmt<E>
where
    E: Ir1IntoIr2<&'a ir0::Ctx, Ir2 = Expr>,
{
    type Ir2 = Stmt;

    fn into_ir2(self, ctx: &'a ir0::Ctx) -> Self::Ir2 {
        Stmt::Let {
            pattern: self.pattern.into_ir2(ctx),
            expr: self.expr.into_ir2(ctx),
        }
    }
}

impl Ir1IntoIr2<&'_ ir0::Ctx> for ir1::stream::Expr {
    type Ir2 = Expr;

    fn into_ir2(self, ctx: &ir0::Ctx) -> Self::Ir2 {
        use ir1::stream::Kind::*;
        match self.kind {
            ComponentApplication {
                memory_id,
                called_comp_id,
                inputs,
            } => {
                let memory_ident = ctx
                    .get_name(memory_id.expect(
                        "internal error: should be defined in `ir1::stream::Expr::memorize`",
                    ))
                    .clone();
                let name = ctx.get_name(called_comp_id).clone();
                let input_fields = inputs
                    .into_iter()
                    .map(|(id, expression)| (ctx.get_name(id).clone(), expression.into_ir2(ctx)))
                    .collect_vec();
                let path_opt = ctx.try_get_comp_path(called_comp_id);
                let outputs = ctx
                    .get_comp_outputs(called_comp_id)
                    .iter()
                    .map(|(_, id)| ctx.get_name(*id).clone());
                ir2::Expr::comp_call(memory_ident, name, input_fields, outputs, path_opt.cloned())
            }
            Expression { expr } => expr.into_ir2(ctx),
            SomeEvent { expr } => ir2::Expr::some(expr.into_ir2(ctx)),
            NoneEvent => ir2::Expr::none(),
            Last { ident_id, .. } => {
                let name = ctx.get_name(ident_id).clone();
                ir2::Expr::MemoryAccess { identifier: name }
            }
            WhenExpr { .. } | RisingEdge { .. } => noErrorDesc!(),
        }
    }

    fn try_get_typ(&self) -> Option<&Typ> {
        self.typ.as_ref()
    }

    fn is_if_then_else(&self, ctx: &ir0::Ctx) -> bool {
        match &self.kind {
            ir1::stream::Kind::Expression { expr } => expr.is_if_then_else(ctx),
            _ => false,
        }
    }
}

impl Ir1IntoIr2<&'_ ir0::Ctx> for ir1::Typedef {
    type Ir2 = Item;

    fn into_ir2(self, ctx: &ir0::Ctx) -> Self::Ir2 {
        use ir1::typedef::Kind;
        match self.kind {
            Kind::Structure { fields, .. } => Item::Structure(Structure {
                name: ctx.get_name(self.id).clone(),
                fields: fields
                    .into_iter()
                    .map(|id| (ctx.get_name(id).clone(), ctx.get_typ(id).clone()))
                    .collect(),
            }),
            Kind::Enumeration { elements, .. } => Item::Enumeration(Enumeration {
                name: ctx.get_name(self.id).clone(),
                elements: elements
                    .into_iter()
                    .map(|id| ctx.get_name(id).clone())
                    .collect(),
            }),
            Kind::Array => Item::ArrayAlias(ir2::item::ArrayAlias {
                name: ctx.get_name(self.id).clone(),
                array_type: ctx.get_array_type(self.id).clone(),
                size: ctx.get_array_size(self.id),
            }),
        }
    }
}
