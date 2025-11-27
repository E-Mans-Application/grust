//! [`stream::Expr`][Expr] module.

use grust_compiler_common::itertools::Itertools;

use crate::event_pattern::EventPattern;

prelude! {
    graph::{Label, DiGraphMap},
}

pub type Stmt = ir1::Stmt<Expr>;

mk_new! { impl Stmt => new {
    pattern: ir1::stmt::Pattern,
    expr: Expr,
    loc: Loc,
} }

impl Stmt {
    pub fn get_identifiers(&self) -> Vec<usize> {
        let mut identifiers = match &self.expr.kind {
            stream::Kind::Expression {
                expr: expr::Kind::MatchExpr { arms, .. },
            } => arms
                .iter()
                .flat_map(|(pattern, _, statements, _)| {
                    statements
                        .iter()
                        .flat_map(|statement| statement.get_identifiers())
                        .chain(pattern.identifiers())
                })
                .collect(),
            stream::Kind::WhenExpr { arms, .. } => arms
                .iter()
                .flat_map(|(pattern, _, statements)| {
                    statements
                        .iter()
                        .flat_map(|statement| statement.get_identifiers())
                        .chain(pattern.identifiers())
                })
                .collect(),
            _ => vec![],
        };

        identifiers.append(&mut self.pattern.identifiers());
        identifiers
    }

    /// Increments memory with statement's expression.
    ///
    /// Store component applications.
    ///
    /// # Example
    ///
    /// A statement `x: int = my_comp(s, x_1).o;` increments memory with the component call
    /// `mem_my_comp_o_: (my_comp, o);` and the statement is unchanged.
    ///
    /// Examples are tested in source.
    pub fn memorize(
        &mut self,
        identifier_creator: &mut IdentifierCreator,
        memory: &mut Memory,
        ctx: &mut Ctx,
    ) -> URes {
        self.expr.memorize(identifier_creator, memory, ctx)
    }

    /// Change [ir1] statement into a normal form.
    ///
    /// The normal form of an statement is as follows:
    ///
    /// - component application can only append at root expression
    /// - component application inputs are ident calls
    ///
    /// # Example
    ///
    /// ```GR
    /// x: int = 1 + my_comp(s, v*2).o;
    /// ```
    ///
    /// The above example becomes:
    ///
    /// ```GR
    /// x_1: int = v*2;
    /// x_2: int = my_comp(s, x_1).o;
    /// x: int = 1 + x_2;
    /// ```
    pub fn normal_form(
        mut self,
        components_reduced_graphs: &HashMap<usize, DiGraphMap<usize, Label>>,
        identifier_creator: &mut IdentifierCreator,
        ctx: &mut Ctx,
    ) -> (Vec<stream::Stmt>, Vec<stream::InitStmt>) {
        // change expression into normal form and get additional statements
        let (mut new_stmts, new_inits) = match self.expr.kind {
            stream::Kind::ComponentApplication {
                called_comp_id,
                ref mut inputs,
                ..
            } => {
                let (mut new_stmts, mut new_inits) = (vec![], vec![]);
                for (_, expr) in inputs.iter_mut() {
                    let (add_stmts, add_inits) =
                        expr.into_ident_call(components_reduced_graphs, identifier_creator, ctx);
                    new_stmts.extend(add_stmts);
                    new_inits.extend(add_inits);
                }

                // change dependencies to be the sum of inputs dependencies
                let reduced_graph = components_reduced_graphs.get(&called_comp_id).unwrap();
                self.expr.dependencies = Dependencies::from(
                    inputs
                        .iter()
                        .flat_map(|(input_id, expr)| {
                            ctx.get_comp_outputs(called_comp_id).iter().flat_map(
                                |(_, output_id)| {
                                    reduced_graph
                                        .edge_weight(*output_id, *input_id)
                                        .into_iter()
                                        .flat_map(|label1| {
                                            expr.get_dependencies()
                                                .clone()
                                                .into_iter()
                                                .map(|(id, label2)| (id, label1.add(&label2)))
                                        })
                                },
                            )
                        })
                        .collect(),
                );

                (new_stmts, new_inits)
            }
            _ => self
                .expr
                .normal_form(components_reduced_graphs, identifier_creator, ctx),
        };

        // push normal_formed self statement in the statements storage
        new_stmts.push(self);

        // return statements
        (new_stmts, new_inits)
    }

    pub fn add_to_graph(&self, graph: &mut DiGraphMap<usize, Label>) {
        let idents = self.pattern.identifiers();
        for from in idents.iter() {
            for (to, label) in self.expr.get_dependencies() {
                graph::add_edge(graph, *from, *to, *label);
            }
        }
        if let stream::Kind::Expression {
            expr: ir1::expr::Kind::MatchExpr { arms, .. },
        } = &self.expr.kind
        {
            arms.iter().for_each(|(_, bound, statements, _)| {
                if let Some(bound) = bound {
                    for from in idents.iter() {
                        for (to, label) in bound.get_dependencies() {
                            graph::add_edge(graph, *from, *to, *label);
                        }
                    }
                }
                statements
                    .iter()
                    .for_each(|statement| statement.add_to_graph(graph))
            })
        }
    }

    /// Add the statement identifier to the identifier creator.
    ///
    /// It will add the statement identifier to the identifier creator. If the identifier already
    /// exists, then the new identifier created by the identifier creator will be added to the
    /// renaming context.
    pub fn add_necessary_renaming(
        &self,
        identifier_creator: &mut IdentifierCreator,
        context_map: &mut HashMap<usize, Either<usize, stream::Expr>>,
        ctx: &mut Ctx,
    ) {
        // create fresh identifiers for the new statement
        let local_idents = self.pattern.identifiers();
        for ident_id in local_idents {
            let name = ctx.get_name(ident_id);
            let scope = ctx.get_scope(ident_id).clone();
            let fresh_name = identifier_creator.new_identifier(name.span(), name.to_string());
            if Scope::Output != scope && &fresh_name != name {
                let typ = Some(ctx.get_typ(ident_id).clone());
                let fresh_id = ctx.insert_fresh_ident(fresh_name, scope, typ);
                let _unique = context_map.insert(ident_id, Either::Left(fresh_id));
                debug_assert!(_unique.is_none());
            }
        }
    }

    /// Replace identifier occurrence by element in context.
    ///
    /// It will return a new statement where the expression has been modified according to the
    /// context:
    ///
    /// - if an identifier is mapped to another identifier, then rename all occurrence of the
    ///   identifier by the new one
    /// - if the identifier is mapped to an expression, then replace all call to the identifier by
    ///   the expression
    ///
    /// # Example
    ///
    /// With a context `[x -> a, y -> b/2, z -> c]`, a call to the function with the statement `z =
    /// x + y` will return `c = a + b/2`.
    pub fn replace_by_context(
        &self,
        context_map: &HashMap<usize, Either<usize, stream::Expr>>,
    ) -> stream::Stmt {
        let mut new_statement = self.clone();

        // replace statement's identifiers by the new ones
        let local_idents = new_statement.pattern.identifiers_mut();
        for ident_id in local_idents {
            if let Some(element) = context_map.get(ident_id) {
                match element {
                    Either::Left(new_id)
                    | Either::Right(stream::Expr {
                        kind:
                            stream::Kind::Expression {
                                expr: ir1::expr::Kind::Identifier { id: new_id },
                            },
                        ..
                    }) => {
                        *ident_id = *new_id;
                    }
                    Either::Right(_) => noErrorDesc!(),
                }
            }
        }

        // replace identifiers in statement's expression
        new_statement.expr.replace_by_context(context_map);

        new_statement
    }

    /// Inline component application when it is needed.
    ///
    /// Inlining needed for "shifted causality loop".
    ///
    /// # Example:
    /// ```GR
    /// component semi_fib(i: int) -> (o: int) {
    ///     init i = 1;
    ///     init aux = 0;
    ///     let aux: int = i + last i;
    ///     out o: int = last aux;
    /// }
    /// ```
    /// In this example, an statement `fib: int = semi_fib(fib).o` calls
    /// `semi_fib` with the same input and output ident.
    /// There is no causality loop, `o` depends on the memory of `i`.
    ///
    /// We need to inline the code, the output `fib` is defined before the input `fib`,
    /// which can not be computed by a function call.
    pub fn inline_when_needed_recursive(
        self,
        subgraph: &DiGraphMap<usize, Label>,
        memory: &mut Memory,
        identifier_creator: &mut IdentifierCreator,
        ctx: &mut Ctx,
        components: &HashMap<usize, Component>,
    ) -> Vec<stream::Stmt> {
        let mut current_statements = vec![self.clone()];
        let mut new_statements =
            self.inline_when_needed(subgraph, memory, identifier_creator, ctx, components);
        while current_statements != new_statements {
            current_statements = new_statements;
            new_statements = current_statements
                .clone()
                .into_iter()
                .flat_map(|statement| {
                    statement.inline_when_needed(
                        subgraph,
                        memory,
                        identifier_creator,
                        ctx,
                        components,
                    )
                })
                .collect();
        }
        new_statements
    }

    fn inline_when_needed(
        self,
        subgraph: &DiGraphMap<usize, Label>,
        memory: &mut Memory,
        identifier_creator: &mut IdentifierCreator,
        ctx: &mut Ctx,
        components: &HashMap<usize, Component>,
    ) -> Vec<stream::Stmt> {
        match &self.expr.kind {
            stream::Kind::ComponentApplication {
                called_comp_id,
                inputs,
                memory_id,
                ..
            } => {
                // a loop in the graph induces that "component call" inputs depends on output
                let is_loop = {
                    let mut graph = subgraph.clone();
                    let outs = self.pattern.identifiers();
                    let in_deps = inputs.iter().flat_map(|(_, expr)| expr.get_dependencies());
                    for (to, label) in in_deps {
                        for from in outs.iter() {
                            graph.add_edge(*from, *to, *label);
                        }
                    }
                    graph::toposort(&graph, None).is_err()
                };

                // then component call must be inlined
                if is_loop {
                    let called_comp = components.get(called_comp_id).unwrap();

                    // get statements from called component, with corresponding inputs
                    let (retrieved_statements, retrieved_memory) = called_comp
                        .instantiate_statements_and_memory(
                            identifier_creator,
                            inputs,
                            Some(self.pattern),
                            ctx,
                        );

                    // remove called component from memory
                    memory.remove_called_comp(memory_id.unwrap());

                    memory.combine(retrieved_memory);
                    retrieved_statements
                } else {
                    // otherwise, just return self
                    vec![self]
                }
            }
            _ => vec![self],
        }
    }
}

#[derive(Debug, Clone)]
/// Initialization statement.
pub struct InitStmt {
    /// Pattern of elements to initialize.
    pub pattern: stmt::Pattern,
    /// The expression initializing the element.
    pub expr: Expr,
    /// Identifers to the initialized memories.
    pub init_ids: HashMap<usize, usize>,
    /// InitStmt location.
    pub loc: Loc,
}
impl PartialEq for InitStmt {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern && self.expr == other.expr
    }
}
impl InitStmt {
    pub fn get_identifiers(&self) -> Vec<usize> {
        let mut identifiers = match &self.expr.kind {
            stream::Kind::Expression {
                expr: expr::Kind::MatchExpr { arms, .. },
            } => arms
                .iter()
                .flat_map(|(pattern, _, statements, _)| {
                    statements
                        .iter()
                        .flat_map(|statement| statement.get_identifiers())
                        .chain(pattern.identifiers())
                })
                .collect(),
            _ => vec![],
        };

        identifiers.append(&mut self.pattern.identifiers());
        identifiers
    }

    /// Increments memory with initializations.
    pub fn memorize(self, memory: &mut Memory, ctx: &mut Ctx) -> URes {
        // add pattern to memory
        self.expr.store_in_memory(self.pattern, memory, ctx)
    }
}

pub type ExprKind = expr::Kind<Expr>;

impl ExprKind {
    /// Increment memory with expression.
    ///
    /// Store component applications.
    ///
    /// # Example
    ///
    /// An expression `my_comp(s, x_1).o;` increments memory with the component call `mem_my_comp_o_:
    /// (my_comp, o);` and is unchanged.
    ///
    /// Examples are tested in source.
    pub fn memorize(
        &mut self,
        identifier_creator: &mut IdentifierCreator,
        memory: &mut Memory,
        ctx: &mut Ctx,
    ) -> URes {
        match self {
            Self::Constant { .. }
            | Self::Identifier { .. }
            | Self::Lambda { .. }
            | Self::Enumeration { .. } => (),
            Self::UnOp { expr, .. } => {
                expr.memorize(identifier_creator, memory, ctx)?;
            }
            Self::BinOp { lft, rgt, .. } => {
                lft.memorize(identifier_creator, memory, ctx)?;
                rgt.memorize(identifier_creator, memory, ctx)?;
            }
            Self::IfThenElse { cnd, thn, els } => {
                cnd.memorize(identifier_creator, memory, ctx)?;
                thn.memorize(identifier_creator, memory, ctx)?;
                els.memorize(identifier_creator, memory, ctx)?;
            }
            Self::Application { fun, inputs } => {
                fun.memorize(identifier_creator, memory, ctx)?;
                for expr in inputs.iter_mut() {
                    expr.memorize(identifier_creator, memory, ctx)?;
                }
            }
            Self::Structure { fields, .. } => {
                for (_, expr) in fields.iter_mut() {
                    expr.memorize(identifier_creator, memory, ctx)?;
                }
            }
            Self::Array { elements } | Self::Tuple { elements } => {
                for expr in elements {
                    expr.memorize(identifier_creator, memory, ctx)?;
                }
            }
            Self::MatchExpr { expr, arms } => {
                expr.memorize(identifier_creator, memory, ctx)?;
                for (_, option, block, expr) in arms.iter_mut() {
                    if let Some(expr) = option.as_mut() {
                        expr.memorize(identifier_creator, memory, ctx)?;
                    }
                    for statement in block.iter_mut() {
                        statement.memorize(identifier_creator, memory, ctx)?;
                    }
                    expr.memorize(identifier_creator, memory, ctx)?;
                }
            }
            Self::FieldAccess { expr, .. }
            | Self::TupleElementAccess { expr, .. }
            | Self::ArrayAccess { expr, .. } => {
                expr.memorize(identifier_creator, memory, ctx)?;
            }
            Self::Map { expr, fun } => {
                expr.memorize(identifier_creator, memory, ctx)?;
                fun.memorize(identifier_creator, memory, ctx)?;
            }
            Self::Fold { array, init, fun } => {
                array.memorize(identifier_creator, memory, ctx)?;
                init.memorize(identifier_creator, memory, ctx)?;
                fun.memorize(identifier_creator, memory, ctx)?;
            }
            Self::Sort { expr, fun } => {
                expr.memorize(identifier_creator, memory, ctx)?;
                fun.memorize(identifier_creator, memory, ctx)?;
            }
            Self::Zip { arrays } => {
                for expr in arrays {
                    expr.memorize(identifier_creator, memory, ctx)?;
                }
            }
        }
        Ok(())
    }

    /// Change [ir1] expression into a normal form.
    ///
    /// The normal form of an expression is as follows:
    /// - component application can only append at root expression
    /// - component application inputs are ident calls
    ///
    /// # Example
    ///
    /// ```GR
    /// x: int = 1 + my_comp(s, v*2).o;
    /// ```
    ///
    /// The above example becomes:
    ///
    /// ```GR
    /// x_1: int = v*2;
    /// x_2: int = my_comp(s, x_1).o;
    /// x: int = 1 + x_2;
    /// ```
    pub fn normal_form(
        &mut self,
        dependencies: &mut Dependencies,
        components_reduced_graphs: &HashMap<usize, DiGraphMap<usize, Label>>,
        identifier_creator: &mut IdentifierCreator,
        ctx: &mut Ctx,
    ) -> (Vec<stream::Stmt>, Vec<stream::InitStmt>) {
        match self {
            Self::Constant { .. }
            | Self::Identifier { .. }
            | Self::Enumeration { .. }
            | Self::Lambda { .. } => (vec![], vec![]),
            Self::UnOp { expr, .. } => {
                let (new_stmts, new_inits) =
                    expr.normal_form(components_reduced_graphs, identifier_creator, ctx);

                *dependencies = Dependencies::from(expr.get_dependencies().clone());

                (new_stmts, new_inits)
            }

            Self::BinOp { lft, rgt, .. } => {
                let (mut new_stmts, mut new_inits) =
                    lft.normal_form(components_reduced_graphs, identifier_creator, ctx);
                let (stmts, inits) =
                    rgt.normal_form(components_reduced_graphs, identifier_creator, ctx);
                new_stmts.extend(stmts);
                new_inits.extend(inits);

                let mut expression_dependencies = lft.get_dependencies().clone();
                let mut other_dependencies = rgt.get_dependencies().clone();
                expression_dependencies.append(&mut other_dependencies);

                *dependencies = Dependencies::from(expression_dependencies);

                (new_stmts, new_inits)
            }

            Self::IfThenElse { cnd, thn, els } => {
                let (mut new_stmts, mut new_inits) =
                    cnd.normal_form(components_reduced_graphs, identifier_creator, ctx);
                let (stmts, inits) =
                    thn.normal_form(components_reduced_graphs, identifier_creator, ctx);
                new_stmts.extend(stmts);
                new_inits.extend(inits);
                let (stmts, inits) =
                    els.normal_form(components_reduced_graphs, identifier_creator, ctx);
                new_stmts.extend(stmts);
                new_inits.extend(inits);

                let mut expr_dependencies = cnd.get_dependencies().clone();
                let mut other_dependencies = thn.get_dependencies().clone();
                expr_dependencies.append(&mut other_dependencies);
                let mut other_dependencies = els.get_dependencies().clone();
                expr_dependencies.append(&mut other_dependencies);

                *dependencies = Dependencies::from(expr_dependencies);

                (new_stmts, new_inits)
            }

            Self::Application { ref mut inputs, .. } => {
                let (mut new_stmts, mut new_inits) = (vec![], vec![]);
                for expr in inputs.iter_mut() {
                    let (add_stmts, add_inits) =
                        expr.normal_form(components_reduced_graphs, identifier_creator, ctx);
                    new_stmts.extend(add_stmts);
                    new_inits.extend(add_inits);
                }

                *dependencies = Dependencies::from(
                    inputs
                        .iter()
                        .flat_map(|expr| expr.get_dependencies().clone())
                        .collect(),
                );

                (new_stmts, new_inits)
            }

            Self::Structure { fields, .. } => {
                let (mut new_stmts, mut new_inits) = (vec![], vec![]);
                for (_, expr) in fields.iter_mut() {
                    let (add_stmts, add_inits) =
                        expr.normal_form(components_reduced_graphs, identifier_creator, ctx);
                    new_stmts.extend(add_stmts);
                    new_inits.extend(add_inits);
                }

                *dependencies = Dependencies::from(
                    fields
                        .iter()
                        .flat_map(|(_, expression)| expression.get_dependencies().clone())
                        .collect(),
                );

                (new_stmts, new_inits)
            }
            Self::Array { elements } | Self::Tuple { elements } => {
                let (mut new_stmts, mut new_inits) = (vec![], vec![]);
                for expr in elements.iter_mut() {
                    let (add_stmts, add_inits) =
                        expr.normal_form(components_reduced_graphs, identifier_creator, ctx);
                    new_stmts.extend(add_stmts);
                    new_inits.extend(add_inits);
                }

                *dependencies = Dependencies::from(
                    elements
                        .iter()
                        .flat_map(|expression| expression.get_dependencies().clone())
                        .collect(),
                );

                (new_stmts, new_inits)
            }
            Self::MatchExpr { expr, arms, .. } => {
                let (mut new_stmts, mut new_inits) =
                    expr.normal_form(components_reduced_graphs, identifier_creator, ctx);
                let mut expr_dependencies = expr.get_dependencies().clone();

                arms.iter_mut()
                    .for_each(|(pattern, bound, body, matched_expr)| {
                        // get local idents defined in pattern
                        let local_idents = pattern.identifiers();

                        // normalize body statements
                        *body = body
                            .iter_mut()
                            .flat_map(|statement| {
                                let (stmts, inits) = statement.clone().normal_form(
                                    components_reduced_graphs,
                                    identifier_creator,
                                    ctx,
                                );
                                debug_assert!(inits.is_empty());
                                stmts
                            })
                            .collect();

                        // remove identifiers created by the pattern from the dependencies
                        if let Some(expr) = bound.as_mut() {
                            let (stmts, inits) = expr.normal_form(
                                components_reduced_graphs,
                                identifier_creator,
                                ctx,
                            );
                            new_stmts.extend(stmts);
                            new_inits.extend(inits);

                            expr_dependencies.extend(
                                expr.get_dependencies()
                                    .clone()
                                    .into_iter()
                                    .filter(|(ident, _)| !local_idents.contains(ident)),
                            )
                        }

                        let (stmts, inits) = matched_expr.normal_form(
                            components_reduced_graphs,
                            identifier_creator,
                            ctx,
                        );
                        body.extend(stmts);
                        debug_assert!(inits.is_empty());

                        expr_dependencies.extend(
                            matched_expr
                                .get_dependencies()
                                .clone()
                                .into_iter()
                                .filter(|(ident, _)| !local_idents.contains(ident)),
                        );
                    });

                *dependencies = Dependencies::from(expr_dependencies);

                (new_stmts, new_inits)
            }
            Self::FieldAccess { expr, .. }
            | Self::TupleElementAccess { expr, .. }
            | Self::ArrayAccess { expr, .. } => {
                let (new_stmts, new_inits) =
                    expr.normal_form(components_reduced_graphs, identifier_creator, ctx);

                *dependencies = Dependencies::from(expr.get_dependencies().clone());

                (new_stmts, new_inits)
            }
            Self::Map { expr, .. } => {
                let (new_stmts, new_inits) =
                    expr.normal_form(components_reduced_graphs, identifier_creator, ctx);

                *dependencies = Dependencies::from(expr.get_dependencies().clone());

                (new_stmts, new_inits)
            }
            Self::Fold { array, init, .. } => {
                let (mut new_stmts, mut new_inits) =
                    array.normal_form(components_reduced_graphs, identifier_creator, ctx);
                let (stmts, inits) =
                    init.normal_form(components_reduced_graphs, identifier_creator, ctx);
                new_stmts.extend(stmts);
                new_inits.extend(inits);

                // get matched expressions dependencies
                let mut expr_dependencies = array.get_dependencies().clone();
                let mut init_dependencies = array.get_dependencies().clone();
                expr_dependencies.append(&mut init_dependencies);

                // push all dependencies in arms dependencies
                *dependencies = Dependencies::from(expr_dependencies);

                (new_stmts, new_inits)
            }
            Self::Sort { expr, .. } => {
                let (new_stmts, new_inits) =
                    expr.normal_form(components_reduced_graphs, identifier_creator, ctx);

                *dependencies = Dependencies::from(expr.get_dependencies().clone());

                (new_stmts, new_inits)
            }
            Self::Zip { arrays, .. } => {
                let (mut new_stmts, mut new_inits) = (vec![], vec![]);
                for expr in arrays.iter_mut() {
                    let (add_stmts, add_inits) =
                        expr.normal_form(components_reduced_graphs, identifier_creator, ctx);
                    new_stmts.extend(add_stmts);
                    new_inits.extend(add_inits);
                }

                *dependencies = Dependencies::from(
                    arrays
                        .iter()
                        .flat_map(|array| array.get_dependencies().clone())
                        .collect(),
                );

                (new_stmts, new_inits)
            }
        }
    }

    /// Replace identifier occurrence by element in context.
    ///
    /// It will modify the expression according to the context:
    ///
    /// - if an identifier is mapped to another identifier, then rename all occurrence of the
    ///   identifier by the new one
    /// - if the identifier is mapped to an expression, then replace all call to the identifier by
    ///   the expression
    ///
    /// # Example
    ///
    /// With a context `[x -> a, y -> b/2]`, the expression `x + y` will become `a + b/2`.
    pub fn replace_by_context(
        &mut self,
        dependencies: &mut Dependencies,
        context_map: &HashMap<usize, Either<usize, stream::Expr>>,
    ) -> Option<stream::Expr> {
        match self {
            Self::Constant { .. } | Self::Lambda { .. } | Self::Enumeration { .. } => None,
            Self::Identifier { ref mut id } => {
                if let Some(element) = context_map.get(id) {
                    match element {
                        Either::Left(new_id) => {
                            *id = *new_id;
                            *dependencies = Dependencies::from(vec![(*new_id, Label::Weight(0))]);
                            None
                        }
                        Either::Right(new_expression) => Some(new_expression.clone()),
                    }
                } else {
                    None
                }
            }
            Self::UnOp { expr, .. } => {
                expr.replace_by_context(context_map);
                *dependencies = Dependencies::from(expr.get_dependencies().clone());
                None
            }
            Self::BinOp { lft, rgt, .. } => {
                lft.replace_by_context(context_map);
                rgt.replace_by_context(context_map);

                let mut expression_dependencies = lft.get_dependencies().clone();
                let mut other_dependencies = rgt.get_dependencies().clone();
                expression_dependencies.append(&mut other_dependencies);

                *dependencies = Dependencies::from(expression_dependencies);
                None
            }
            Self::IfThenElse { cnd, thn, els } => {
                cnd.replace_by_context(context_map);
                thn.replace_by_context(context_map);
                els.replace_by_context(context_map);

                let mut expr_dependencies = cnd.get_dependencies().clone();
                let mut other_dependencies = thn.get_dependencies().clone();
                expr_dependencies.append(&mut other_dependencies);
                let mut other_dependencies = els.get_dependencies().clone();
                expr_dependencies.append(&mut other_dependencies);

                *dependencies = Dependencies::from(expr_dependencies);
                None
            }
            Self::Application { ref mut inputs, .. } => {
                inputs
                    .iter_mut()
                    .for_each(|expression| expression.replace_by_context(context_map));

                *dependencies = Dependencies::from(
                    inputs
                        .iter()
                        .flat_map(|expression| expression.get_dependencies().clone())
                        .collect(),
                );
                None
            }
            Self::Structure { ref mut fields, .. } => {
                fields
                    .iter_mut()
                    .for_each(|(_, expression)| expression.replace_by_context(context_map));

                *dependencies = Dependencies::from(
                    fields
                        .iter()
                        .flat_map(|(_, expression)| expression.get_dependencies().clone())
                        .collect(),
                );
                None
            }
            Self::Array { ref mut elements } | Self::Tuple { ref mut elements } => {
                elements
                    .iter_mut()
                    .for_each(|expression| expression.replace_by_context(context_map));

                *dependencies = Dependencies::from(
                    elements
                        .iter()
                        .flat_map(|expression| expression.get_dependencies().clone())
                        .collect(),
                );
                None
            }
            Self::MatchExpr {
                ref mut expr,
                ref mut arms,
                ..
            } => {
                expr.replace_by_context(context_map);
                let mut expr_dependencies = expr.get_dependencies().clone();

                arms.iter_mut()
                    .for_each(|(pattern, bound, body, matched_expr)| {
                        let local_idents = pattern.identifiers();

                        // remove identifiers created by the pattern from the context
                        let context_map = context_map
                            .clone()
                            .into_iter()
                            .filter(|(key, _)| !local_idents.contains(key))
                            .collect();

                        if let Some(expr) = bound.as_mut() {
                            expr.replace_by_context(&context_map);
                            let mut bound_dependencies = expr
                                .get_dependencies()
                                .clone()
                                .into_iter()
                                .filter(|(ident, _)| !local_idents.contains(ident))
                                .collect();
                            expr_dependencies.append(&mut bound_dependencies);
                        }

                        body.iter_mut()
                            .for_each(|statement| statement.expr.replace_by_context(&context_map));

                        matched_expr.replace_by_context(&context_map);
                        let matched_expr_dependencies = matched_expr
                            .get_dependencies()
                            .clone()
                            .into_iter()
                            .filter(|(ident, _)| !local_idents.contains(ident));
                        expr_dependencies.extend(matched_expr_dependencies);
                    });

                *dependencies = Dependencies::from(expr_dependencies);
                None
            }
            Self::FieldAccess { ref mut expr, .. }
            | Self::TupleElementAccess { ref mut expr, .. }
            | Self::ArrayAccess { ref mut expr, .. } => {
                expr.replace_by_context(context_map);
                // get matched expression dependencies
                let expr_dependencies = expr.get_dependencies().clone();
                // push all dependencies in arms dependencies
                *dependencies = Dependencies::from(expr_dependencies);
                None
            }
            Self::Map { ref mut expr, .. } => {
                expr.replace_by_context(context_map);
                // get matched expression dependencies
                let expr_dependencies = expr.get_dependencies().clone();
                // push all dependencies in arms dependencies
                *dependencies = Dependencies::from(expr_dependencies);
                None
            }
            Self::Fold {
                ref mut array,
                ref mut init,
                ..
            } => {
                array.replace_by_context(context_map);
                init.replace_by_context(context_map);
                // get matched expressions dependencies
                let mut expr_dependencies = array.get_dependencies().clone();
                let mut init_dependencies = array.get_dependencies().clone();
                expr_dependencies.append(&mut init_dependencies);

                // push all dependencies in arms dependencies
                *dependencies = Dependencies::from(expr_dependencies);
                None
            }
            Self::Sort { ref mut expr, .. } => {
                expr.replace_by_context(context_map);
                // get matched expression dependencies
                let expr_dependencies = expr.get_dependencies().clone();
                // push all dependencies in arms dependencies
                *dependencies = Dependencies::from(expr_dependencies);
                None
            }
            Self::Zip { ref mut arrays, .. } => {
                arrays
                    .iter_mut()
                    .for_each(|array| array.replace_by_context(context_map));

                *dependencies = Dependencies::from(
                    arrays
                        .iter()
                        .flat_map(|array| array.get_dependencies().clone())
                        .collect(),
                );
                None
            }
        }
    }
}

/// Type for the arms of the when expression.
pub type Arm = (EventPattern, Option<Expr>, Vec<Stmt>);

#[derive(Debug, PartialEq, Clone)]
/// GRust stream expression kind AST.
pub enum Kind {
    /// Expression.
    Expression {
        /// The expression kind.
        expr: ExprKind,
    },
    /// Initialized buffer stream expression.
    Last {
        /// The initialization id.
        init_id: usize,
        /// The id of ident that is initialized.
        ident_id: usize,
    },
    /// Component application stream expression.
    ComponentApplication {
        /// Component's id in memory.
        memory_id: Option<usize>,
        /// Called component's id in Symbol Table.
        called_comp_id: usize,
        /// The inputs to the expression.
        inputs: Vec<(usize, Expr)>,
    },
    /// When expression.
    WhenExpr {
        def_eq_pat: stmt::Pattern,
        /// The initialization.
        init_ids: HashMap<usize, usize>,
        /// The different matching cases.
        arms: Vec<Arm>,
    },
    /// Detect a rising edge of the expression.
    RisingEdge {
        /// The expression to detect the rising edge from.
        expr: Box<Expr>,
    },
    /// Present event expression.
    SomeEvent {
        /// The expression of the event.
        expr: Box<Expr>,
    },
    /// Absent event expression.
    NoneEvent,
}

impl Kind {
    pub fn is_default_constant(&self) -> bool {
        if let Self::Expression { expr } = self {
            expr.is_default_constant()
        } else {
            false
        }
    }
}

mk_new! { impl Kind =>
    Expression: expr {
        expr: impl Into<ExprKind> = expr.into(),
    }
    Last: last { init_id: usize, ident_id: usize }
    ComponentApplication: call {
        memory_id = None,
        called_comp_id: usize,
        inputs: Vec<(usize, Expr)>,
    }
    WhenExpr: when {
        def_eq_pat: stmt::Pattern,
        init_ids: HashMap<usize, usize>,
        arms: impl Into<Vec<Arm>> = arms.into(),
    }
    RisingEdge: rising_edge {
        expr: Expr = expr.into(),
    }
    SomeEvent: emit { expr: Expr = expr.into() }
    NoneEvent: none_event ()
}

impl HasWeight for Kind {
    fn weight(&self, wb: &synced::WeightBounds, ctx: &Ctx) -> synced::Weight {
        use synced::weight;
        use Kind::*;
        match self {
            Expression { expr } => expr.weight(wb, ctx),
            Last { .. } => weight::lo,
            ComponentApplication { inputs, .. } => {
                // we **don't** want to rayon-ize component applications as it would (seem to) borrow
                // the component's state multiple time in the rayon-closure: we need the weight to be
                // at least `wb.rayon_ubx`
                wb.rayon_ubx() + w8!(sum inputs, |(_, e)| e.weight(wb, ctx))
            }
            WhenExpr { arms, .. } => arms
                .iter()
                .map(|(_, expr_opt, stmts)| {
                    w8!(wb, ctx => weight? expr_opt.as_ref()) + w8!(wb, ctx => sum stmts)
                })
                .max()
                .unwrap_or(weight::zero),
            RisingEdge { expr } => expr.weight(wb, ctx) + weight::mid,
            SomeEvent { expr } => expr.weight(wb, ctx) + weight::mid,
            NoneEvent => weight::zero,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// GRust stream expression AST.
pub struct Expr {
    /// Stream expression kind.
    pub kind: Kind,
    /// Stream expression type.
    pub typ: Option<Typ>,
    /// Stream expression location.
    pub loc: Loc,
    /// Stream expression dependencies.
    pub dependencies: Dependencies,
}
impl HasLoc for Expr {
    fn loc(&self) -> Loc {
        self.loc
    }
}

impl Expr {
    /// Constructs stream expression.
    ///
    /// Typing, location and dependencies are empty.
    pub fn new(loc: impl Into<Loc>, kind: Kind) -> Expr {
        Expr {
            kind,
            typ: None,
            loc: loc.into(),
            dependencies: Dependencies::new(),
        }
    }

    pub fn is_default_constant(&self) -> bool {
        self.kind.is_default_constant()
    }

    /// Get stream expression's type.
    pub fn get_type(&self) -> Option<&Typ> {
        self.typ.as_ref()
    }
    /// Get stream expression's mutable type.
    pub fn get_type_mut(&mut self) -> Option<&mut Typ> {
        self.typ.as_mut()
    }

    /// Get stream expression's dependencies.
    pub fn get_dependencies(&self) -> &Vec<(usize, Label)> {
        self.dependencies
            .get()
            .expect("internal error: there should be dependencies")
    }

    /// Tell if it is in normal form.
    ///
    /// - no when expressions
    /// - component application as root expression
    /// - no rising edge
    pub fn is_normal_form(&self) -> bool {
        let predicate_expr = |expr: &Expr| expr.no_comp_application() && expr.no_rising_edge();
        let predicate_statement = |statement: &Stmt| statement.expr.is_normal_form();
        match &self.kind {
            Kind::Expression { expr } => {
                expr.propagate_predicate(predicate_expr, predicate_statement)
            }
            Kind::Last { .. } => true,
            Kind::ComponentApplication { inputs, .. } => {
                inputs.iter().all(|(_, expr)| predicate_expr(expr))
            }
            Kind::WhenExpr { .. } => false,
            Kind::SomeEvent { expr } => predicate_expr(expr),
            Kind::NoneEvent => true,
            Kind::RisingEdge { .. } => false,
        }
    }

    /// Tell if there is no component application.
    pub fn no_comp_application(&self) -> bool {
        match &self.kind {
            Kind::Expression { expr } => expr
                .propagate_predicate(Self::no_comp_application, |statement| {
                    statement.expr.no_comp_application()
                }),
            Kind::Last { .. } => true,
            Kind::ComponentApplication { .. } => false,
            Kind::WhenExpr { arms, .. } => arms.iter().all(|(_, expr_opt, stmts)| {
                let expr_opt_test = expr_opt
                    .as_ref()
                    .is_none_or(|expr| expr.no_comp_application());
                let stmts_test = stmts.iter().all(|stmt| stmt.no_comp_application());
                expr_opt_test && stmts_test
            }),
            Kind::SomeEvent { expr } => expr.no_comp_application(),
            Kind::NoneEvent => true,
            Kind::RisingEdge { expr } => expr.no_comp_application(),
        }
    }

    /// Tell if there is no rising edge.
    pub fn no_rising_edge(&self) -> bool {
        match &self.kind {
            Kind::Expression { expr } => expr
                .propagate_predicate(Self::no_rising_edge, |statement| {
                    statement.expr.no_rising_edge()
                }),
            Kind::Last { .. } => true,
            Kind::ComponentApplication { inputs, .. } => {
                inputs.iter().all(|(_, expr)| expr.no_rising_edge())
            }
            Kind::WhenExpr { arms, .. } => arms.iter().all(|(_, expr_opt, stmts)| {
                let expr_opt_test = expr_opt.as_ref().is_none_or(|expr| expr.no_rising_edge());
                let stmts_test = stmts.iter().all(|stmt| stmt.no_rising_edge());
                expr_opt_test && stmts_test
            }),
            Kind::SomeEvent { expr } => expr.no_rising_edge(),
            Kind::NoneEvent => true,
            Kind::RisingEdge { .. } => false,
        }
    }

    /// Increment memory with expression.
    ///
    /// Store component applications.
    ///
    /// # Example
    ///
    /// An expression `my_comp(s, x_1).o;` increments memory with the component call `mem_   my_comp_o_:
    /// (my_comp, o);` and is unchanged.
    ///
    /// Examples are tested in source.
    pub fn memorize(
        &mut self,
        identifier_creator: &mut IdentifierCreator,
        memory: &mut Memory,
        ctx: &mut Ctx,
    ) -> URes {
        match &mut self.kind {
            stream::Kind::Expression { expr } => {
                expr.memorize(identifier_creator, memory, ctx)?;
            }
            stream::Kind::Last { .. } => (),
            stream::Kind::ComponentApplication {
                called_comp_id,
                memory_id: comp_memory_id,
                ..
            } => {
                debug_assert!(comp_memory_id.is_none());
                // create fresh identifier for the new memory buffer
                let comp_name = ctx.get_name(*called_comp_id);
                let memory_name =
                    identifier_creator.new_identifier(comp_name.loc(), comp_name.to_string());
                let memory_id = ctx.insert_fresh_ident(memory_name, Scope::Local, None);
                memory.add_called_comp(memory_id, *called_comp_id);
                // put the 'memory_id' of the called component
                *comp_memory_id = Some(memory_id);
            }
            stream::Kind::SomeEvent { expr } => {
                expr.memorize(identifier_creator, memory, ctx)?;
            }
            stream::Kind::NoneEvent => (),
            Kind::WhenExpr { .. } | stream::Kind::RisingEdge { .. } => noErrorDesc!(),
        }
        Ok(())
    }

    pub fn store_in_memory(
        mut self,
        mut pat: stmt::Pattern,
        memory: &mut Memory,
        ctx: &mut Ctx,
    ) -> URes {
        match (self.kind, pat.kind) {
            (
                Kind::Expression {
                    expr:
                        expr::Kind::Tuple {
                            elements: expr_elem,
                        },
                },
                stmt::Kind::Tuple {
                    elements: pat_elems,
                },
            ) => {
                res_vec!(expr_elem
                    .into_iter()
                    .zip(pat_elems.into_iter())
                    .map(|(expr, pat)| expr.store_in_memory(pat, memory, ctx)));
                Ok(())
            }
            (a, b) => {
                self.kind = a;
                pat.kind = b;
                let id = pat
                    .identifiers()
                    .pop()
                    .expect("internal error: should be checked at typing");
                // add buffer to memory
                let name = ctx.get_name(id);
                let typ = ctx.get_typ(id);
                memory.add_buffer(id, name.clone(), typ.clone(), self)
            }
        }
    }

    /// Change [ir1] expression into a normal form.
    ///
    /// The normal form of an expression is as follows:
    /// - component application can only append at root expression
    /// - component application inputs are ident calls
    ///
    /// # Example
    ///
    /// ```GR
    /// x: int = 1 + my_comp(s, v*2).o;
    /// ```
    ///
    /// The above example becomes:
    ///
    /// ```GR
    /// x_1: int = v*2;
    /// x_2: int = my_comp(s, x_1).o;
    /// x: int = 1 + x_2;
    /// ```
    pub fn normal_form(
        &mut self,
        components_reduced_graphs: &HashMap<usize, DiGraphMap<usize, Label>>,
        identifier_creator: &mut IdentifierCreator,
        ctx: &mut Ctx,
    ) -> (Vec<stream::Stmt>, Vec<stream::InitStmt>) {
        let loc = self.loc;
        match self.kind {
            stream::Kind::Last { .. } => (vec![], vec![]),
            stream::Kind::RisingEdge { ref mut expr } => {
                let (new_stmts, mut new_inits) =
                    expr.into_ident_call(components_reduced_graphs, identifier_creator, ctx);
                if let stream::Kind::Expression {
                    expr: expr::Kind::Identifier { id },
                } = expr.kind
                {
                    let last_dependencies = Dependencies::from(vec![(id, Label::weight(1))]);
                    let init_pat = stmt::Pattern {
                        kind: stmt::Kind::ident(id),
                        typ: Some(Typ::Boolean(Default::default())),
                        loc,
                    };
                    let init_expr = stream::Expr {
                        kind: stream::Kind::expr(expr::Kind::constant(Constant::bool(
                            syn::LitBool::new(false, macro2::Span::call_site()),
                        ))),
                        typ: Some(Typ::Boolean(Default::default())),
                        loc,
                        dependencies: Dependencies::from(vec![]),
                    };
                    let init_id = id;
                    let init_mem = stream::InitStmt {
                        pattern: init_pat,
                        expr: init_expr,
                        init_ids: HashMap::from_iter([(id, init_id)]),
                        loc,
                    };
                    new_inits.push(init_mem);

                    let mem = stream::Expr {
                        kind: stream::Kind::last(init_id, id),
                        typ: Some(Typ::Boolean(Default::default())),
                        loc,
                        dependencies: last_dependencies.clone(),
                    };
                    let not_mem = stream::Expr {
                        kind: stream::Kind::expr(expr::Kind::unop(UOp::Not, mem)),
                        typ: Some(Typ::Boolean(Default::default())),
                        loc,
                        dependencies: last_dependencies,
                    };

                    self.dependencies =
                        Dependencies::from(vec![(id, Label::weight(0)), (id, Label::weight(1))]);
                    self.kind =
                        stream::Kind::expr(expr::Kind::binop(BOp::And, *expr.clone(), not_mem)); // TODO: put dependencies, type, and reactive type?

                    (new_stmts, new_inits)
                } else {
                    noErrorDesc!("internal error: rising edge should be detected on ident only")
                }
            }
            stream::Kind::ComponentApplication {
                called_comp_id,
                ref mut inputs,
                ..
            } => {
                let (mut new_stmts, mut new_inits) = (vec![], vec![]);
                for (_, expr) in inputs.iter_mut() {
                    let (add_stmts, add_inits) =
                        expr.into_ident_call(components_reduced_graphs, identifier_creator, ctx);
                    new_stmts.extend(add_stmts);
                    new_inits.extend(add_inits);
                }

                // change dependencies to be the sum of inputs dependencies
                let reduced_graph = components_reduced_graphs.get(&called_comp_id).unwrap();
                self.dependencies = Dependencies::from(
                    inputs
                        .iter()
                        .flat_map(|(input_id, expr)| {
                            ctx.get_comp_outputs(called_comp_id).iter().flat_map(
                                |(_, output_id)| {
                                    reduced_graph
                                        .edge_weight(*output_id, *input_id)
                                        .into_iter()
                                        .flat_map(|label1| {
                                            expr.get_dependencies()
                                                .clone()
                                                .into_iter()
                                                .map(|(id, label2)| (id, label1.add(&label2)))
                                        })
                                },
                            )
                        })
                        .collect(),
                );

                // create fresh identifier for the new statement
                let fresh_name = identifier_creator.fresh_identifier(
                    loc,
                    "comp_app",
                    ctx.get_name(called_comp_id).to_string(),
                );
                let typ = self.get_type().cloned();
                let fresh_id = ctx.insert_fresh_ident(fresh_name, Scope::Local, typ.clone());

                // create statement for component call
                let comp_application_statement = Stmt {
                    pattern: ir1::stmt::Pattern {
                        kind: ir1::stmt::Kind::Identifier { id: fresh_id },
                        typ,
                        loc: self.loc,
                    },
                    expr: self.clone(),
                    loc: self.loc,
                };
                new_stmts.push(comp_application_statement);

                // change current expression be an identifier to the statement of the component call
                self.kind = stream::Kind::Expression {
                    expr: expr::Kind::Identifier { id: fresh_id },
                };
                self.dependencies = Dependencies::from(vec![(fresh_id, Label::Weight(0))]);

                // return new additional statements
                (new_stmts, new_inits)
            }
            stream::Kind::Expression { ref mut expr } => expr.normal_form(
                &mut self.dependencies,
                components_reduced_graphs,
                identifier_creator,
                ctx,
            ),
            stream::Kind::WhenExpr {
                ref mut def_eq_pat,
                ref init_ids,
                ref mut arms,
            } => {
                let (
                    // default patterns for events detection
                    dflt_event_elems,
                    // map from event_id to index in tuple pattern
                    events_indices,
                ) = {
                    // create map from event_id to index in tuple pattern
                    let mut events_indices = HashMap::with_capacity(arms.len());
                    let mut dflt_event_elems = Vec::with_capacity(arms.len() * 2);
                    let mut idx = 0;
                    for (ref pattern, _, _) in arms.iter() {
                        let prev_idx = idx;
                        pattern.place_events(&mut events_indices, &mut idx);
                        for _ in prev_idx..idx {
                            // put default pattern for unlooked events
                            dflt_event_elems.push(Pattern::new(
                                pattern.loc(),
                                pattern::Kind::default(pattern.loc()),
                            ));
                        }
                    }
                    dflt_event_elems.shrink_to_fit();
                    (dflt_event_elems, events_indices)
                };

                let mut inits = vec![];
                let mut stmts = vec![];
                let mut match_arms = vec![];
                // for each arm construct [ir1] pattern, guard and statements
                for (event_pattern, expr_opt, arm_stmts) in arms {
                    // create matched pattern
                    let (matched_pattern, guard) = {
                        let mut elements = dflt_event_elems.clone();
                        let opt_rising_edges = event_pattern
                            .clone()
                            .create_tuple_pattern(&mut elements, &events_indices);
                        let matched =
                            Pattern::new(event_pattern.loc(), pattern::Kind::tuple(elements));

                        let mut guard = expr_opt.clone();
                        // add rising edge detection to the guard
                        if let Some(rising_edges) = opt_rising_edges {
                            if let Some(old_guard) = guard.take() {
                                guard = Some(ir1::stream::Expr::new(
                                    old_guard.loc(),
                                    ir1::stream::Kind::expr(expr::Kind::binop(
                                        BOp::And,
                                        old_guard,
                                        rising_edges,
                                    )),
                                ));
                            } else {
                                guard = Some(rising_edges)
                            }
                        };

                        // normalize guard
                        if let Some(expr) = guard.as_mut() {
                            let (new_stmts, new_inits) = expr.normal_form(
                                components_reduced_graphs,
                                identifier_creator,
                                ctx,
                            );
                            // add inits
                            inits.extend(new_inits);
                            // add stmts
                            stmts.extend(new_stmts);
                        }

                        (matched, guard)
                    };

                    // normalize arm_stmts
                    let mut new_arms_stmts: Vec<ir1::Stmt<Expr>> = vec![];
                    let mut idents = vec![];
                    for stmt in arm_stmts.drain(..) {
                        // collect defined ids
                        idents.extend(
                            stmt.get_identifiers()
                                .iter()
                                .map(|id| ctx.get_name(*id).clone()),
                        );
                        // normalize stmt
                        let (new_stmts, new_inits) =
                            stmt.normal_form(components_reduced_graphs, identifier_creator, ctx);
                        // add inits
                        inits.extend(new_inits);
                        // add new_arms_stmts
                        new_arms_stmts.extend(new_stmts);
                    }
                    let mut not_updated_signals: Vec<ir1::Stmt<Expr>> = vec![];
                    for (id, init_id) in init_ids {
                        let name = ctx.get_name(*id);
                        if !idents.contains(name) {
                            let last_stmt = ir1::Stmt::new(
                                ir1::stmt::Pattern::new(loc, ir1::stmt::Kind::ident(*id)),
                                ir1::stream::Expr {
                                    kind: ir1::stream::Kind::last(*init_id, *id),
                                    typ: Some(ctx.get_typ(*id).clone()),
                                    loc,
                                    dependencies: Dependencies::from(vec![(*id, Label::weight(1))]),
                                },
                                loc,
                            );
                            not_updated_signals.push(last_stmt);
                            idents.push(name.clone());
                        }
                    }
                    not_updated_signals.extend(new_arms_stmts);

                    // create the tuple expression
                    let tuple_expr = def_eq_pat.default_expr(&idents, init_ids, ctx);

                    match_arms.push((matched_pattern, guard, not_updated_signals, tuple_expr))
                }

                // default arm
                let dflt_arm = {
                    // create tuple pattern
                    let elements = dflt_event_elems.clone();
                    let pattern = Pattern::new(self.loc, pattern::Kind::tuple(elements));
                    let guard = None;
                    // create the tuple expression
                    let expr = def_eq_pat.default_expr(&vec![], init_ids, ctx);
                    debug_assert!(expr.is_normal_form());

                    (pattern, guard, vec![], expr)
                };
                match_arms.push(dflt_arm);

                // construct the match expression
                let match_expr = {
                    // create tuple expression to match
                    let tuple_expr = {
                        let elements = events_indices
                            .iter()
                            .sorted_by_key(|(_, idx)| **idx)
                            .map(|(event_id, idx)| {
                                ir1::stream::Expr::new(
                                    match_arms[*idx].0.loc(),
                                    ir1::stream::Kind::expr(ir1::expr::Kind::ident(*event_id)),
                                )
                            })
                            .collect();
                        ir1::stream::Expr::new(
                            self.loc,
                            ir1::stream::Kind::expr(ir1::expr::Kind::tuple(elements)),
                        )
                    };
                    ir1::stream::Kind::expr(ir1::expr::Kind::match_expr(tuple_expr, match_arms))
                };

                self.kind = match_expr;
                (stmts, inits)
            }
            stream::Kind::SomeEvent { ref mut expr } => {
                let (new_stmts, new_inits) =
                    expr.normal_form(components_reduced_graphs, identifier_creator, ctx);
                self.dependencies = Dependencies::from(expr.get_dependencies().clone());
                (new_stmts, new_inits)
            }
            stream::Kind::NoneEvent => (vec![], vec![]),
        }
    }

    /// Change [ir1] expression into a ident call.
    ///
    /// If the expression is not a ident call, then normalize the expression,
    /// create an statement with the normalized expression and change current
    /// expression into a call to the statement.
    ///
    /// # Example
    ///
    /// The expression `1 + my_comp(s, v*2).o` becomes `x` along with:
    ///
    /// ```GR
    /// x_1: int = v*2;
    /// x_2: int = my_comp(s, x_1).o;
    /// x: int = 1 + x_2;
    /// ```
    pub fn into_ident_call(
        &mut self,
        components_reduced_graphs: &HashMap<usize, DiGraphMap<usize, Label>>,
        identifier_creator: &mut IdentifierCreator,
        ctx: &mut Ctx,
    ) -> (Vec<stream::Stmt>, Vec<stream::InitStmt>) {
        match self.kind {
            stream::Kind::Expression {
                expr: expr::Kind::Identifier { .. },
            } => (vec![], vec![]),
            _ => {
                let (mut new_stmts, new_inits) =
                    self.normal_form(components_reduced_graphs, identifier_creator, ctx);

                // create fresh identifier for the new statement
                let fresh_name = identifier_creator.fresh_identifier(self.loc(), "", "x");
                let typ = self.get_type();
                let fresh_id = ctx.insert_fresh_ident(fresh_name, Scope::Local, typ.cloned());

                // create statement for the expression
                let new_statement = Stmt {
                    pattern: ir1::stmt::Pattern {
                        kind: ir1::stmt::Kind::Identifier { id: fresh_id },
                        typ: typ.cloned(),
                        loc: self.loc,
                    },
                    loc: self.loc,
                    expr: self.clone(),
                };
                new_stmts.push(new_statement);

                // change current expression be an identifier to the statement of the expression
                self.kind = stream::Kind::Expression {
                    expr: expr::Kind::Identifier { id: fresh_id },
                };
                self.dependencies = Dependencies::from(vec![(fresh_id, Label::Weight(0))]);

                // return new additional statements
                (new_stmts, new_inits)
            }
        }
    }

    /// Replace identifier occurrence by element in context.
    ///
    /// It will modify the expression according to the context:
    ///
    /// - if an identifier is mapped to another identifier, then rename all occurrence of the
    ///   identifier by the new one
    /// - if the identifier is mapped to an expression, then replace all call to the identifier by
    ///   the expression
    ///
    /// # Example
    ///
    /// With a context `[x -> a, y -> b/2]`, the expression `x + y` will become `a + b/2`.
    pub fn replace_by_context(
        &mut self,
        context_map: &HashMap<usize, Either<usize, stream::Expr>>,
    ) {
        match self.kind {
            stream::Kind::Expression { ref mut expr } => {
                let option_new_expr = expr.replace_by_context(&mut self.dependencies, context_map);
                if let Some(new_expr) = option_new_expr {
                    *self = new_expr;
                }
            }
            stream::Kind::ComponentApplication {
                ref mut memory_id,
                ref mut inputs,
                ..
            } => {
                // replace the id of the called component
                if let Some(element) = context_map.get(&memory_id.unwrap()) {
                    match element {
                        Either::Left(new_id)
                        | Either::Right(stream::Expr {
                            kind:
                                stream::Kind::Expression {
                                    expr: ir1::expr::Kind::Identifier { id: new_id },
                                },
                            ..
                        }) => {
                            *memory_id = Some(*new_id);
                        }
                        Either::Right(_) => noErrorDesc!(),
                    }
                }

                inputs
                    .iter_mut()
                    .for_each(|(_, expr)| expr.replace_by_context(context_map));

                // change dependencies to be the sum of inputs dependencies
                self.dependencies = Dependencies::from(
                    inputs
                        .iter()
                        .flat_map(|(_, expr)| expr.get_dependencies().clone())
                        .collect(),
                );
            }
            stream::Kind::SomeEvent { ref mut expr } => {
                expr.replace_by_context(context_map);

                // change dependencies to be the sum of inputs dependencies
                self.dependencies = Dependencies::from(expr.get_dependencies().clone());
            }
            stream::Kind::NoneEvent => (),
            stream::Kind::Last {
                ref mut ident_id,
                ref mut init_id,
            } => {
                if let Some(element) = context_map.get(ident_id) {
                    match element {
                        Either::Left(new_id)
                        | Either::Right(stream::Expr {
                            kind:
                                stream::Kind::Expression {
                                    expr: ir1::expr::Kind::Identifier { id: new_id },
                                },
                            ..
                        }) => {
                            *ident_id = *new_id;
                            self.dependencies =
                                Dependencies::from(vec![(*new_id, Label::Weight(1))]);
                        }
                        Either::Right(_) => noErrorDesc!(),
                    }
                }
                if let Some(element) = context_map.get(init_id) {
                    match element {
                        Either::Left(new_id)
                        | Either::Right(stream::Expr {
                            kind:
                                stream::Kind::Expression {
                                    expr: ir1::expr::Kind::Identifier { id: new_id },
                                },
                            ..
                        }) => {
                            *init_id = *new_id;
                        }
                        Either::Right(_) => noErrorDesc!(),
                    }
                }
            }
            stream::Kind::WhenExpr { .. } | stream::Kind::RisingEdge { .. } => noErrorDesc!(),
        }
    }
}

impl HasWeight for Expr {
    fn weight(&self, wb: &synced::WeightBounds, ctx: &Ctx) -> synced::Weight {
        self.kind.weight(wb, ctx)
    }
}
