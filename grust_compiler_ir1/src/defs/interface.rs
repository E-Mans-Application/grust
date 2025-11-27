prelude! {
    graph::{DiGraphMap, Direction},
}

pub struct Service {
    /// Service's identifier.
    pub id: usize,
    /// Service's time range `[min, max]` periods.
    pub time_range: Option<(u64, u64)>,
    /// Service's statements.
    pub statements: HashMap<usize, FlowStatement>,
    /// Flows dependency graph, nodes are statements ids.
    pub graph: DiGraphMap<usize, ()>,
}
impl Service {
    pub fn get_flows_names<'a>(&'a self, ctx: &'a Ctx) -> impl Iterator<Item = Ident> + 'a {
        self.statements
            .values()
            .flat_map(|statement| match statement {
                FlowStatement::Declaration(FlowDeclaration { pattern, .. })
                | FlowStatement::Instantiation(FlowInstantiation { pattern, .. }) => pattern
                    .identifiers()
                    .into_iter()
                    .map(|id| ctx.get_name(id).clone()),
            })
    }

    pub fn get_flows_ids<'a>(
        &'a self,
        imports: impl Iterator<Item = &'a FlowImport> + 'a,
    ) -> impl Iterator<Item = usize> + 'a {
        self.statements
            .values()
            .flat_map(|statement| match statement {
                FlowStatement::Declaration(FlowDeclaration { pattern, .. })
                | FlowStatement::Instantiation(FlowInstantiation { pattern, .. }) => {
                    pattern.identifiers()
                }
            })
            .chain(imports.map(|import| import.id))
    }

    pub fn get_flows_context<'a>(
        &self,
        ctx: &Ctx,
        imports: impl Iterator<Item = &'a FlowImport> + 'a,
        exports: impl Iterator<Item = &'a FlowExport> + 'a,
    ) -> ctx::Flows {
        let mut flows_context = ctx::Flows {
            elements: Default::default(),
        };
        self.statements
            .values()
            .for_each(|statement| statement.add_flows_context(&mut flows_context, ctx));
        imports
            .map(|import| import.id)
            .chain(exports.map(|export| export.id))
            .for_each(|id| {
                // if signal push flow in context
                let flow_name = ctx.get_name(id).clone();
                let ty = ctx.get_typ(id);
                if ctx.get_flow_kind(id).is_signal() {
                    flows_context.add_element(flow_name.clone(), ty);
                }
            });
        flows_context
    }

    pub fn get_dependencies(&self, stmt_id: usize) -> impl Iterator<Item = usize> + '_ {
        self.graph
            .edges_directed(stmt_id, Direction::Incoming)
            .map(|(incoming, _, _)| incoming)
    }

    /// Compute the dependency graph of the service.
    ///
    /// # Example
    ///
    /// ```GR
    /// statement 1) import signal s;
    /// statement 2) import event e;
    /// statement 3) export signal o;                // depends on statement 5)
    ///
    /// statement 4) let event e2 = timeout(e, 30);  // depends on statement 2)
    /// statement 5) o = my_comp(s, e2);        // depends on statement 1) and 4)
    /// ```
    pub fn compute_dependencies(
        &mut self,
        flows_imports: &HashMap<usize, usize>,
        flows_exports: &HashMap<usize, usize>,
    ) {
        // create map from flow id to statement defining the flow
        let flows_statements = self.flow_id_to_statement_id(flows_imports);

        // initiate graph
        let mut graph = self.create_initialized_graph(&flows_statements, flows_exports);

        // complete dependency graphs
        self.statements.iter().for_each(|(stmt_id, statement)| {
            statement.add_dependencies(*stmt_id, &flows_statements, &mut graph);
        });

        // set service's graph
        self.graph = graph;
    }

    /// Create a map from flow identifier to its definition statement.
    ///
    /// Export statements do not define the flow, it is the instantiation statement that defines it.
    /// Then, when a flow is exported, it is the instantiation statement that is linked to the flow
    /// identifier in the map.
    fn flow_id_to_statement_id(
        &self,
        flows_imports: &HashMap<usize, usize>,
    ) -> HashMap<usize, usize> {
        let mut flows_statements = flows_imports.clone();

        self.statements.iter().for_each(|(stmt_id, statement)| {
            match &statement {
                FlowStatement::Declaration(FlowDeclaration { pattern, .. })
                | FlowStatement::Instantiation(FlowInstantiation { pattern, .. }) => {
                    pattern.identifiers().into_iter().for_each(|id| {
                        flows_statements.insert(id, *stmt_id);
                    });
                }
            };
        });

        flows_statements
    }

    /// Create an initialized graph from a service.
    ///
    /// The created graph has every statements' indexes as vertices.
    /// But no edges are added.
    fn create_initialized_graph(
        &self,
        flows_statements: &HashMap<usize, usize>,
        flows_exports: &HashMap<usize, usize>,
    ) -> DiGraphMap<usize, ()> {
        // create an empty graph
        let mut graph = DiGraphMap::new();

        // add service's statements as vertices
        for stmt_id in flows_statements.values() {
            graph.add_node(*stmt_id);
        }

        // add potential dependencies between export and service's statements
        flows_exports.iter().for_each(|(flow_id, export_id)| {
            if let Some(stmt_id) = flows_statements.get(flow_id) {
                graph.add_edge(*stmt_id, *export_id, ());
            }
        });

        // return graph
        graph
    }

    pub fn normal_form(&mut self, ctx: &mut Ctx) {
        ctx.local();
        let mut identifier_creator = IdentifierCreator::from(self.get_flows_names(ctx));
        let statements = std::mem::take(&mut self.statements);
        debug_assert!(self.statements.is_empty());
        statements.into_values().for_each(|flow_statement| {
            let statements = flow_statement.normal_form(&mut identifier_creator, ctx);
            for statement in statements {
                let _unique = self.statements.insert(ctx.get_fresh_id(), statement);
                debug_assert!(_unique.is_none())
            }
        });
        ctx.global()
    }

    /// Create memory identifiers for [ir1] components called by service.
    pub fn memorize(&mut self, ctx: &mut Ctx) -> URes {
        // create an IdentifierCreator, a local Ctx and Memory
        let mut identifier_creator = IdentifierCreator::from(self.get_flows_names(ctx));
        ctx.local();

        for statement in self.statements.values_mut() {
            let kind = &mut statement.get_expr_mut().kind;
            kind.memorize(&mut identifier_creator, ctx)?;
        }

        // drop IdentifierCreator (auto) and local Ctx
        ctx.global();

        Ok(())
    }
}

pub struct Interface {
    /// stmt_id -> import
    pub imports: HashMap<usize, FlowImport>,
    /// stmt_id -> export
    pub exports: HashMap<usize, FlowExport>,
    /// GRust interface's services.
    pub services: Vec<Service>,
}

impl Interface {
    /// Generate dependency graphs for every services.
    #[inline]
    pub fn generate_flows_dependency_graphs(&mut self) {
        let flows_imports = self.flow_id_to_import_id();
        let flows_exports = self.flow_id_to_export_id();
        self.services
            .iter_mut()
            .for_each(|service| service.compute_dependencies(&flows_imports, &flows_exports))
    }

    /// Create a map from flow identifier to its import statement.
    pub fn flow_id_to_import_id(&self) -> HashMap<usize, usize> {
        let mut flows_imports = HashMap::new();
        self.imports.iter().for_each(|(stmt_id, import)| {
            flows_imports.insert(import.id, *stmt_id);
        });
        flows_imports
    }

    /// Create a map from flow identifier to its export statement.
    pub fn flow_id_to_export_id(&self) -> HashMap<usize, usize> {
        let mut flows_exports = HashMap::new();
        self.exports.iter().for_each(|(stmt_id, export)| {
            flows_exports.insert(export.id, *stmt_id);
        });
        flows_exports
    }

    pub fn normal_form(&mut self, ctx: &mut Ctx) {
        self.services
            .iter_mut()
            .for_each(|service| service.normal_form(ctx))
    }
}

/// Flow statement.
#[derive(Clone)]
pub struct FlowDeclaration {
    pub let_token: Token![let],
    /// Pattern of flows and their types.
    pub pattern: ir1::stmt::Pattern,
    pub eq_token: Token![=],
    /// The expression defining the flow.
    pub expr: ir1::flow::Expr,
    pub semi_token: Token![;],
}
/// Flow statement.
#[derive(Clone)]
pub struct FlowInstantiation {
    /// Pattern of flows and their types.
    pub pattern: ir1::stmt::Pattern,
    pub eq_token: Token![=],
    /// The expression defining the flow.
    pub expr: ir1::flow::Expr,
    pub semi_token: Token![;],
}
/// Flow statement.
#[derive(Clone)]
pub struct FlowImport {
    pub token: Either<keyword::import, keyword::timer>,
    /// Identifier of the flow and its type.
    pub id: usize,
    pub path: syn::Path,
    pub colon_token: Token![:],
    pub flow_type: Typ,
    pub semi_token: Token![;],
}
/// Flow statement.
#[derive(Clone)]
pub struct FlowExport {
    pub export_token: keyword::export,
    /// Identifier of the flow and its type.
    pub id: usize,
    pub path: syn::Path,
    pub colon_token: Token![:],
    pub flow_type: Typ,
    pub semi_token: Token![;],
}

#[derive(Clone)]
pub enum FlowStatement {
    Declaration(FlowDeclaration),
    Instantiation(FlowInstantiation),
}
impl FlowStatement {
    pub fn get_expr_mut(&mut self) -> &mut ir1::flow::Expr {
        match self {
            FlowStatement::Declaration(flow_declaration) => &mut flow_declaration.expr,
            FlowStatement::Instantiation(flow_instantiation) => &mut flow_instantiation.expr,
        }
    }

    /// Retrieves the component index and its inputs if the statement contains an invocation.
    pub fn try_get_call(&self) -> Option<&Vec<(usize, ir1::flow::Expr)>> {
        use FlowStatement::*;
        match self {
            Declaration(FlowDeclaration {
                expr:
                    ir1::flow::Expr {
                        kind: ir1::flow::Kind::ComponentCall { inputs, .. },
                        ..
                    },
                ..
            })
            | Instantiation(FlowInstantiation {
                expr:
                    ir1::flow::Expr {
                        kind: ir1::flow::Kind::ComponentCall { inputs, .. },
                        ..
                    },
                ..
            }) => Some(inputs),
            Declaration(_) | Instantiation(_) => None,
        }
    }

    /// Tells if the statement is a component call.
    pub fn is_comp_call(&self) -> bool {
        use FlowStatement::*;
        match self {
            Declaration(FlowDeclaration {
                expr:
                    ir1::flow::Expr {
                        kind: ir1::flow::Kind::ComponentCall { .. },
                        ..
                    },
                ..
            })
            | Instantiation(FlowInstantiation {
                expr:
                    ir1::flow::Expr {
                        kind: ir1::flow::Kind::ComponentCall { .. },
                        ..
                    },
                ..
            }) => true,
            Declaration(_) | Instantiation(_) => false,
        }
    }

    /// Retrieves the identifiers the statement defines.
    pub fn get_identifiers(&self) -> Vec<usize> {
        use FlowStatement::*;
        match self {
            Declaration(FlowDeclaration { pattern, .. })
            | Instantiation(FlowInstantiation { pattern, .. }) => pattern.identifiers(),
        }
    }

    /// Retrieves the statement's dependencies.
    pub fn get_dependencies(&self) -> Vec<usize> {
        match self {
            FlowStatement::Declaration(FlowDeclaration { expr, .. })
            | FlowStatement::Instantiation(FlowInstantiation { expr, .. }) => {
                expr.get_dependencies()
            }
        }
    }

    pub fn add_dependencies(
        &self,
        stmt_id: usize,
        flows_statements: &HashMap<usize, usize>,
        graph: &mut DiGraphMap<usize, ()>,
    ) {
        match self {
            FlowStatement::Declaration(FlowDeclaration { expr, .. })
            | FlowStatement::Instantiation(FlowInstantiation { expr, .. }) => {
                debug_assert!(expr.is_normal());
                let dependencies = expr.get_dependencies();
                dependencies.iter().for_each(|flow_id| {
                    let dep_id = flows_statements
                        .get(flow_id)
                        .expect("internal error: should be there");
                    graph.add_edge(*dep_id, stmt_id, ());
                });
            }
        }
    }

    /// Change flow statement into a normal form.
    ///
    /// The normal form of a flow expression is as follows:
    /// - flow expressions others than identifiers are root expression
    /// - then, arguments are only identifiers
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
        identifier_creator: &mut IdentifierCreator,
        ctx: &mut Ctx,
    ) -> Vec<FlowStatement> {
        let mut new_statements = match &mut self {
            FlowStatement::Declaration(FlowDeclaration { ref mut expr, .. })
            | FlowStatement::Instantiation(FlowInstantiation { ref mut expr, .. }) => {
                expr.normal_form(identifier_creator, ctx)
            }
        };
        new_statements.push(self);
        new_statements
    }
    fn add_flows_context(&self, flows_context: &mut ctx::Flows, ctx: &Ctx) {
        match self {
            FlowStatement::Declaration(FlowDeclaration { pattern, expr, .. })
            | FlowStatement::Instantiation(FlowInstantiation { pattern, expr, .. }) => {
                match &expr.kind {
                    flow::Kind::Throttle { .. } => {
                        // get the id of pattern's flow (and check their is only one flow)
                        let mut ids = pattern.identifiers();
                        debug_assert!(ids.len() == 1);
                        let pattern_id = ids.pop().unwrap();

                        // push in flows context
                        let flow_name = ctx.get_name(pattern_id).clone();
                        let ty = ctx.get_typ(pattern_id);
                        flows_context.add_element(flow_name.clone(), ty);
                    }
                    flow::Kind::Persist { .. } => {
                        // get the id of pattern's flow (and check their is only one flow)
                        let mut ids = pattern.identifiers();
                        debug_assert!(ids.len() == 1);
                        let pattern_id = ids.pop().unwrap();

                        // push in flows context
                        let flow_name = ctx.get_name(pattern_id).clone();
                        let ty = ctx.get_typ(pattern_id);
                        flows_context.add_element(flow_name.clone(), ty);
                    }
                    flow::Kind::Sample { expr, .. } | flow::Kind::SampleOn { expr, .. } => {
                        // get the id of expr (and check it is an identifier, from
                        // normalization)
                        let id = match &expr.kind {
                            flow::Kind::Ident { id } => *id,
                            _ => noErrorDesc!(),
                        };
                        // get pattern's id
                        let mut ids = pattern.identifiers();
                        assert!(ids.len() == 1);
                        let pattern_id = ids.pop().unwrap();

                        // push in flows context
                        let source_name = ctx.get_name(id).clone();
                        let flow_name = ctx.get_name(pattern_id).clone();
                        let ty = Typ::option(ctx.get_typ(id).clone());
                        flows_context.add_element(source_name, &ty);
                        flows_context.add_element(flow_name, &ty);
                    }
                    flow::Kind::Scan { expr, .. }
                    | flow::Kind::ScanOn { expr, .. }
                    | flow::Kind::OnChange { expr, .. } => {
                        // get the id of expr (and check it is an identifier, from
                        // normalization)
                        let id = match &expr.kind {
                            flow::Kind::Ident { id } => *id,
                            _ => noErrorDesc!(),
                        };

                        // push in flows context
                        let source_name = ctx.get_name(id).clone();
                        let ty = ctx.get_typ(id);
                        flows_context.add_element(source_name, ty);
                    }
                    flow::Kind::ComponentCall { inputs, .. }
                    | flow::Kind::FunctionCall { inputs, .. } => {
                        // get outputs' ids
                        let outputs_ids = pattern.identifiers();

                        // store output flows in flows_context
                        for output_id in outputs_ids.iter() {
                            let output_name = ctx.get_name(*output_id);
                            let output_type = ctx.get_typ(*output_id);
                            flows_context.add_element(output_name.clone(), output_type)
                        }

                        inputs.iter().for_each(|(_, expr)| {
                            match &expr.kind {
                                // get the id of expr (and check it is an identifier, from
                                // normalization)
                                flow::Kind::Ident { id: flow_id } => {
                                    let flow_name = ctx.get_name(*flow_id).clone();
                                    let ty = ctx.get_typ(*flow_id);
                                    if !ty.is_event() {
                                        // push in context
                                        flows_context.add_element(flow_name, ty);
                                    }
                                }
                                _ => noErrorDesc!(),
                            }
                        });
                    }
                    flow::Kind::Ident { .. }
                    | flow::Kind::Timeout { .. }
                    | flow::Kind::Merge { .. }
                    | flow::Kind::Time { .. }
                    | flow::Kind::Period { .. } => (),
                }
            }
        }
    }
}
