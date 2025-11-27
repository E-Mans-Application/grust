//! [Component] module.

prelude! {
    graph::*,
}

use super::memory::Memory;

#[derive(Debug, Clone, PartialEq)]
/// GRust component.
pub struct Component {
    /// Component's signature
    pub sign: ComponentSignature,
    /// Component's
    pub body_or_path: Either<ComponentBody, syn::Path>,
}
impl Component {
    pub fn new(
        id: usize,
        inits: Vec<ir1::stream::InitStmt>,
        statements: Vec<ir1::stream::Stmt>,
        contract: ir1::Contract,
        logs: Vec<usize>,
        loc: Loc,
    ) -> Self {
        Self {
            sign: ComponentSignature::new(id, loc),
            body_or_path: Either::Left(ComponentBody::new(inits, statements, contract, logs)),
        }
    }

    pub fn new_ext(id: usize, path: syn::Path, loc: Loc) -> Self {
        Self {
            sign: ComponentSignature::new(id, loc),
            body_or_path: Either::Right(path),
        }
    }

    pub fn get_graph(&self) -> Option<&DiGraphMap<usize, Label>> {
        match &self.body_or_path {
            Either::Left(body) => Some(&body.graph),
            Either::Right(_) => None,
        }
    }
    pub fn get_reduced_graph(&self) -> &DiGraphMap<usize, Label> {
        &self.sign.reduced_graph
    }
    pub fn get_id(&self) -> usize {
        self.sign.id
    }
    pub fn get_location(&self) -> Loc {
        self.sign.loc
    }

    /// Check the causality of the component.
    pub fn causal(&self, ctx: &Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        match &self.body_or_path {
            Either::Left(body) => {
                // 0-dependency graph
                let subgraph = body.zero_dependency_graph();

                // if a schedule exists, then the component is causal
                let res = graph::toposort(&subgraph, None);
                if let Err(cycle) = res {
                    let name = ctx.get_name(cycle.node_id());
                    bad!( errors, @self.get_location() => ErrorKind::ident_non_causal(name.to_string()) )
                }

                Ok(())
            }
            Either::Right(_) => Ok(()),
        }
    }

    /// Create memory for [ir1] component.
    ///
    /// Store buffer for last expressions and component applications.
    /// Transform last expressions in ident call.
    pub fn memorize(&mut self, ctx: &mut Ctx) -> URes {
        match &mut self.body_or_path {
            Either::Left(body) => body.memorize(ctx),
            Either::Right(_) => Ok(()),
        }
    }

    /// Change [ir1] component into a normal form.
    ///
    /// The normal form of a component is as follows:
    /// - component application can only append at root expression
    /// - component application inputs are ident calls
    ///
    /// # Example
    ///
    /// ```GR
    /// component test(s: int, v: int, g: int) {
    ///     out x: int = 1 + my_comp(s, v*2).o;
    ///     out y: int = other_comp(g-1, v).o;
    /// }
    /// ```
    ///
    /// The above component contains the following components:
    ///
    /// ```GR
    /// component test_x(s: int, v: int) {
    ///     out x: int = 1 + my_comp(s, v*2).o;
    /// }
    /// component test_y(v: int, g: int) {
    ///     out y: int = other_comp(g-1, v).o;
    /// }
    /// ```
    ///
    /// Which are transformed into:
    ///
    /// ```GR
    /// component test_x(s: int, v: int) {
    ///     x_1: int = v*2;
    ///     x_2: int = my_comp(s, x_1).o;
    ///     out x: int = 1 + x_2;
    /// }
    /// component test_y(v: int, g: int) {
    ///     x: int = g-1;
    ///     out y: int = other_comp(x_1, v).o;
    /// }
    /// ```
    pub fn normal_form(
        &mut self,
        components_reduced_graphs: &HashMap<usize, DiGraphMap<usize, Label>>,
        ctx: &mut Ctx,
    ) {
        match &mut self.body_or_path {
            Either::Left(body) => body.normal_form(components_reduced_graphs, ctx),
            Either::Right(_) => (),
        }
    }

    /// Inline component application when it is needed.
    ///
    /// Inlining needed for "shifted causality loop".
    pub fn inline_when_needed(&mut self, components: &HashMap<usize, Component>, ctx: &mut Ctx) {
        match &mut self.body_or_path {
            Either::Left(body) => body.inline_when_needed(components, ctx),
            Either::Right(_) => (),
        }
    }

    /// Instantiate component's statements with inputs.
    ///
    /// It will return new statements where the input idents are instantiated by expressions. New
    /// statements should have fresh id according to the calling component.
    pub fn instantiate_statements_and_memory(
        &self,
        identifier_creator: &mut IdentifierCreator,
        inputs: &[(usize, stream::Expr)],
        new_output_pattern: Option<stmt::Pattern>,
        ctx: &mut Ctx,
    ) -> (Vec<stream::Stmt>, Memory) {
        match &self.body_or_path {
            Either::Left(body) => {
                // create the context with the given inputs
                let mut context_map = inputs
                    .iter()
                    .map(|(input, expression)| (*input, Either::Right(expression.clone())))
                    .collect::<HashMap<_, _>>();

                // add output idents to context
                if let Some(pattern) = new_output_pattern {
                    let idents = pattern.identifiers();
                    ctx.get_comp_outputs(self.sign.id)
                        .iter()
                        .zip(idents)
                        .for_each(|((_, output_id), new_output_id)| {
                            context_map.insert(*output_id, Either::Left(new_output_id));
                        })
                };

                body.instantiate_statements_and_memory(identifier_creator, context_map, ctx)
            }
            Either::Right(_) => (vec![], Memory::new()),
        }
    }

    /// Schedule statements.
    pub fn schedule(&mut self) {
        match &mut self.body_or_path {
            Either::Left(body) => body.schedule(),
            Either::Right(_) => (),
        }
    }

    /// Tell if it is in normal form.
    pub fn is_normal_form(&self) -> bool {
        match &self.body_or_path {
            Either::Left(body) => body.is_normal_form(),
            Either::Right(_) => true,
        }
    }
}

pub mod dump_graph {
    prelude! { graph::* }
    use grust_compiler_common::{
        json::append_json,
        serde::ser::{SerializeSeq, SerializeStruct},
    };

    impl Component {
        /// Dump dependency graph with parallelization weights.
        pub fn dump_graph<P: AsRef<std::path::Path>>(&self, filepath: P, ctx: &Ctx) {
            if let Either::Left(body) = &self.body_or_path {
                let mut graph = DiGraphMap::new();
                if let conf::ComponentPara::Para(bounds) = ctx.conf.component_para {
                    // map identifiers to the weight of their stmt
                    let weight_map: HashMap<usize, usize> = body
                        .statements
                        .iter()
                        .flat_map(|stmt| {
                            let w = stmt.weight(&bounds, ctx);
                            stmt.get_identifiers().into_iter().map(move |id| (id, w))
                        })
                        .collect();
                    // build another graph, with more infos (such as idents' names, and their weights)
                    for (a, b, label) in body.graph.all_edges() {
                        match label {
                            Label::Contract => (),
                            Label::Weight(memory_depth) => {
                                graph.add_edge(
                                    Flow::new(
                                        a,
                                        ctx.get_name(a),
                                        weight_map.get(&a).map_or(0, |w| *w),
                                    ),
                                    Flow::new(
                                        b,
                                        ctx.get_name(b),
                                        weight_map.get(&b).map_or(0, |w| *w),
                                    ),
                                    *memory_depth,
                                );
                            }
                        }
                    }
                }
                // push in JSON file
                append_json(
                    filepath,
                    ComponentGraph::new(ctx.get_name(self.get_id()), graph),
                );
            }
        }
    }

    #[derive(Debug, Clone, Copy, Eq)]
    pub struct Flow<'a> {
        id: usize,
        name: &'a Ident,
        weight: usize,
    }
    impl<'a> Flow<'a> {
        fn new(id: usize, name: &'a Ident, weight: usize) -> Self {
            Self { id, name, weight }
        }
    }
    impl Hash for Flow<'_> {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.id.hash(state);
        }
    }
    impl PartialEq for Flow<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }
    impl PartialOrd for Flow<'_> {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for Flow<'_> {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.id.cmp(&other.id)
        }
    }
    impl grust_compiler_common::prelude::Serialize for Flow<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut flow = serializer.serialize_struct("Flow", 3)?;
            flow.serialize_field("name", &self.name.to_string())?;
            flow.serialize_field("weight", &self.weight)?;
            flow.end()
        }
    }
    pub struct ComponentGraph<'a> {
        name: &'a Ident,
        graph: DiGraphMap<Flow<'a>, usize>,
    }
    impl<'a> ComponentGraph<'a> {
        fn new(name: &'a Ident, graph: DiGraphMap<Flow<'a>, usize>) -> Self {
            Self { name, graph }
        }
    }
    impl grust_compiler_common::prelude::Serialize for ComponentGraph<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut comp_graph = serializer.serialize_struct("Component", 3)?;
            comp_graph.serialize_field("name", &self.name.to_string())?;
            comp_graph.serialize_field("nodes", &SerializeNodes::new(&self.graph))?;
            comp_graph.serialize_field("edges", &SerializeEdges::new(&self.graph))?;
            comp_graph.end()
        }
    }
    struct SerializeNodes<'a, 'b> {
        graph: &'a DiGraphMap<Flow<'b>, usize>,
    }
    impl<'a, 'b> SerializeNodes<'a, 'b> {
        fn new(graph: &'a DiGraphMap<Flow<'b>, usize>) -> Self {
            Self { graph }
        }
    }
    impl grust_compiler_common::prelude::Serialize for SerializeNodes<'_, '_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(self.graph.node_count()))?;
            for element in self.graph.nodes() {
                seq.serialize_element(&element)?;
            }
            seq.end()
        }
    }
    struct SerializeEdges<'a, 'b> {
        graph: &'a DiGraphMap<Flow<'b>, usize>,
    }
    impl<'a, 'b> SerializeEdges<'a, 'b> {
        fn new(graph: &'a DiGraphMap<Flow<'b>, usize>) -> Self {
            Self { graph }
        }
    }
    impl grust_compiler_common::prelude::Serialize for SerializeEdges<'_, '_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(self.graph.edge_count()))?;
            for (n1, n2, _) in self.graph.all_edges() {
                seq.serialize_element(&(n1.name.to_string(), n2.name.to_string()))?;
            }
            seq.end()
        }
    }
}

#[derive(Debug, Clone)]
/// GRust component definition.
pub struct ComponentBody {
    /// Component's initialization statements.
    pub inits: Vec<ir1::stream::InitStmt>,
    /// Component's statements.
    pub statements: Vec<ir1::stream::Stmt>,
    /// Component's contract.
    pub contract: ir1::Contract,
    /// Logs.
    pub logs: Vec<usize>,
    /// Component's memory.
    pub memory: Memory,
    /// Component dependency graph.
    pub graph: DiGraphMap<usize, Label>,
}

impl PartialEq for ComponentBody {
    fn eq(&self, other: &Self) -> bool {
        self.inits == other.inits
            && self.statements == other.statements
            && self.contract == other.contract
            && self.eq_graph(other)
    }
}

impl ComponentBody {
    pub fn new(
        inits: Vec<ir1::stream::InitStmt>,
        statements: Vec<ir1::stream::Stmt>,
        contract: ir1::Contract,
        logs: Vec<usize>,
    ) -> Self {
        Self {
            inits,
            statements,
            contract,
            logs,
            graph: graph::DiGraphMap::new(),
            memory: ir1::Memory::new(),
        }
    }

    /// Return vector of component's idents id.
    pub fn get_idents_id(&self) -> Vec<usize> {
        self.statements
            .iter()
            .flat_map(|statement| statement.get_identifiers())
            .collect()
    }

    /// Return vector of component's idents name.
    pub fn get_idents_names(&self, ctx: &Ctx) -> Vec<Ident> {
        self.statements
            .iter()
            .flat_map(|statement| statement.get_identifiers())
            .chain(self.memory.get_identifiers().cloned())
            .map(|id| ctx.get_name(id).clone())
            .collect()
    }

    fn eq_graph(&self, other: &ComponentBody) -> bool {
        let graph_comps = self.graph.nodes();
        let other_comps = other.graph.nodes();
        let graph_edges = self.graph.all_edges();
        let other_edges = other.graph.all_edges();
        graph_comps.eq(other_comps) && graph_edges.eq(other_edges)
    }

    /// Tell if it is in normal form.
    pub fn is_normal_form(&self) -> bool {
        self.statements
            .iter()
            .all(|statement| statement.is_normal_form())
    }

    /// Create memory for [ir1] component.
    ///
    /// Store buffer for last expressions and component applications.
    /// Transform last expressions in ident call.
    pub fn memorize(&mut self, ctx: &mut Ctx) -> URes {
        // create an IdentifierCreator, a local Ctx and Memory
        let mut identifier_creator = IdentifierCreator::from(self.get_idents_names(ctx));
        ctx.local();
        let mut memory = Memory::new();

        for init in self.inits.drain(..) {
            init.memorize(&mut memory, ctx)?;
        }

        for statement in self.statements.iter_mut() {
            statement.memorize(&mut identifier_creator, &mut memory, ctx)?;
        }

        self.contract
            .memorize(&mut identifier_creator, &mut memory, ctx);

        // drop IdentifierCreator (auto), local Ctx and set Memory
        ctx.global();
        self.memory = memory;

        // add a dependency graph to the component
        let mut graph = GraphMap::new();
        self.get_idents_id().iter().for_each(|ident_id| {
            graph.add_node(*ident_id);
        });
        self.statements
            .iter()
            .for_each(|statement| statement.add_to_graph(&mut graph));
        self.graph = graph;
        Ok(())
    }

    /// Change [ir1] component into a normal form.
    ///
    /// The normal form of a component is as follows:
    /// - component application can only append at root expression
    /// - component application inputs are ident calls
    ///
    /// # Example
    ///
    /// ```GR
    /// component test(s: int, v: int, g: int) {
    ///     out x: int = 1 + my_comp(s, v*2).o;
    ///     out y: int = other_comp(g-1, v).o;
    /// }
    /// ```
    ///
    /// The above component contains the following components:
    ///
    /// ```GR
    /// component test_x(s: int, v: int) {
    ///     out x: int = 1 + my_comp(s, v*2).o;
    /// }
    /// component test_y(v: int, g: int) {
    ///     out y: int = other_comp(g-1, v).o;
    /// }
    /// ```
    ///
    /// Which are transformed into:
    ///
    /// ```GR
    /// component test_x(s: int, v: int) {
    ///     x_1: int = v*2;
    ///     x_2: int = my_comp(s, x_1).o;
    ///     out x: int = 1 + x_2;
    /// }
    /// component test_y(v: int, g: int) {
    ///     x: int = g-1;
    ///     out y: int = other_comp(x_1, v).o;
    /// }
    /// ```
    pub fn normal_form(
        &mut self,
        components_reduced_graphs: &HashMap<usize, DiGraphMap<usize, Label>>,
        ctx: &mut Ctx,
    ) {
        // create an IdentifierCreator and a local Ctx
        let mut identifier_creator = IdentifierCreator::from(self.get_idents_names(ctx));
        ctx.local();

        for init in &self.inits {
            // initialization expressions should already be in normal form
            debug_assert!(init.expr.is_normal_form());
        }

        let mut new_stmts = vec![];
        for stmt in self.statements.drain(..) {
            let (add_stmts, add_inits) =
                stmt.normal_form(components_reduced_graphs, &mut identifier_creator, ctx);
            new_stmts.extend(add_stmts);
            self.inits.extend(add_inits);
        }
        self.statements = new_stmts;

        // drop IdentifierCreator (auto) and local Ctx
        ctx.global();

        // add a dependency graph to the component
        let mut graph = GraphMap::new();
        self.get_idents_id().iter().for_each(|ident_id| {
            graph.add_node(*ident_id);
        });
        self.statements
            .iter()
            .for_each(|statement| statement.add_to_graph(&mut graph));
        self.graph = graph;
    }

    /// Constructs component's 0-dependency graph.
    fn zero_dependency_graph(&self) -> DiGraphMap<usize, Label> {
        let mut subgraph = self.graph.clone();
        self.graph
            .all_edges()
            .for_each(|(from, to, label)| match label {
                Label::Weight(0) => (),
                _ => {
                    let _label = subgraph.remove_edge(from, to);
                    debug_assert_ne!(_label, Some(Label::Weight(0)))
                }
            });
        subgraph
    }

    /// Inline component application when it is needed.
    ///
    /// Inlining needed for "shifted causality loop".
    pub fn inline_when_needed(&mut self, components: &HashMap<usize, Component>, ctx: &mut Ctx) {
        // create identifier creator containing the idents
        let mut identifier_creator = IdentifierCreator::from(self.get_idents_names(ctx));

        // 0-dependency graph
        let subgraph = self.zero_dependency_graph();

        // compute new statements for the component
        let mut new_statements: Vec<stream::Stmt> = vec![];
        std::mem::take(&mut self.statements)
            .into_iter()
            .for_each(|statement| {
                let mut retrieved_statements = statement.inline_when_needed_recursive(
                    &subgraph,
                    &mut self.memory,
                    &mut identifier_creator,
                    ctx,
                    components,
                );
                new_statements.append(&mut retrieved_statements)
            });

        // update component's stmt
        self.update_statements(&new_statements)
    }

    /// Instantiate component's statements with inputs.
    ///
    /// It will return new statements where the input idents are instantiated by expressions. New
    /// statements should have fresh id according to the calling component.
    pub fn instantiate_statements_and_memory(
        &self,
        identifier_creator: &mut IdentifierCreator,
        mut context_map: HashMap<usize, Either<usize, stream::Expr>>,
        ctx: &mut Ctx,
    ) -> (Vec<stream::Stmt>, Memory) {
        // add identifiers of the inlined statements to the context
        self.statements.iter().for_each(|statement| {
            statement.add_necessary_renaming(identifier_creator, &mut context_map, ctx)
        });
        // add identifiers of the inlined memory to the context
        self.memory
            .add_necessary_renaming(identifier_creator, &mut context_map, ctx);

        // reduce statements according to the context
        let statements = self
            .statements
            .iter()
            .map(|statement| statement.replace_by_context(&context_map))
            .collect();

        // reduce memory according to the context
        let memory = self.memory.replace_by_context(&context_map, ctx);

        (statements, memory)
    }

    /// Update component statements and add the corresponding dependency graph.
    fn update_statements(&mut self, new_statements: &[stream::Stmt]) {
        // put new statements in component
        self.statements = new_statements.to_vec();
        // add a dependency graph to the component
        let mut graph = GraphMap::new();
        self.get_idents_id().iter().for_each(|ident_id| {
            graph.add_node(*ident_id);
        });
        self.statements
            .iter()
            .for_each(|statement| statement.add_to_graph(&mut graph));
        self.graph = graph;
    }

    /// Schedule statements.
    pub fn schedule(&mut self) {
        // get subgraph with only direct dependencies
        let mut subgraph = self.graph.clone();
        self.graph
            .all_edges()
            .for_each(|(from, to, label)| match label {
                Label::Weight(0) => (),
                _ => {
                    let res = subgraph.remove_edge(from, to);
                    debug_assert_ne!(res, Some(Label::Weight(0)))
                }
            });

        // topological sorting
        let mut schedule = toposort(&subgraph, None).unwrap();
        schedule.reverse();

        // construct map from ident_id to their position in the schedule
        let idents_order = schedule
            .into_iter()
            .enumerate()
            .map(|(order, ident_id)| (ident_id, order))
            .collect::<HashMap<_, _>>();
        let compare = |statement: &stream::Stmt| {
            statement
                .pattern
                .identifiers()
                .into_iter()
                .map(|ident_id| idents_order.get(&ident_id).unwrap())
                .min()
                .unwrap()
        };

        // sort statements
        self.statements.sort_by_key(compare);
        self.statements.iter_mut().for_each(|statement| {
            if let stream::Kind::Expression {
                expr: ir1::expr::Kind::MatchExpr { arms, .. },
            } = &mut statement.expr.kind
            {
                arms.iter_mut()
                    .for_each(|(_, _, statements, _)| statements.sort_by_key(compare))
            }
        })
    }
}

#[derive(Debug, Clone)]
/// GRust component import.
pub struct ComponentSignature {
    /// Component identifier.
    pub id: usize,
    /// Component location.
    pub loc: Loc,
    /// Component reduced dependency graph.
    pub reduced_graph: DiGraphMap<usize, Label>,
}

impl PartialEq for ComponentSignature {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.loc == other.loc && self.eq_graph(other)
    }
}

impl ComponentSignature {
    pub fn new(id: usize, loc: Loc) -> Self {
        Self {
            id,
            loc,
            reduced_graph: graph::DiGraphMap::new(),
        }
    }

    fn eq_graph(&self, other: &ComponentSignature) -> bool {
        let graph_comps = self.reduced_graph.nodes();
        let other_comps = other.reduced_graph.nodes();
        let graph_edges = self.reduced_graph.all_edges();
        let other_edges = other.reduced_graph.all_edges();
        graph_comps.eq(other_comps) && graph_edges.eq(other_edges)
    }
}
