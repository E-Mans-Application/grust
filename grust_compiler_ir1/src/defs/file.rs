//! [File] module.

prelude! {}

/// A GRust [File] is composed of functions, components,
/// types defined by the user, components and interface.
pub struct File {
    /// Program types.
    pub typedefs: Vec<ir1::Typedef>,
    /// Program functions.
    pub functions: Vec<ir1::Function>,
    /// Program components. They are functional requirements.
    pub components: Vec<ir1::Component>,
    /// Program interface. It represents the system.
    pub interface: ir1::Interface,
    /// Program location.
    pub loc: Loc,
}

impl File {
    /// Tell if it is in normal form.
    ///
    /// - component application as root expression
    /// - no rising edge
    fn is_normal_form(&self) -> bool {
        self.components
            .iter()
            .all(|component| component.is_normal_form())
    }

    /// Check the causality of the file.
    pub fn causality_analysis(&self, ctx: &Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        // check causality for each component
        self.components
            .iter()
            .map(|component| component.causal(ctx, errors))
            .collect::<Vec<_>>()
            .into_iter()
            .collect::<TRes<_>>()
    }

    /// Create memory for [ir1] components.
    ///
    /// Store buffer for last expressions and component applications.
    /// Transform last expressions in identifier call.
    fn memorize(&mut self, ctx: &mut Ctx) -> URes {
        for comp in self.components.iter_mut() {
            comp.memorize(ctx)?;
        }
        for service in self.interface.services.iter_mut() {
            service.memorize(ctx)?;
        }
        Ok(())
    }

    /// Change [ir1] file into a normal form.
    fn normal_form(&mut self, ctx: &mut Ctx) {
        let mut components_reduced_graphs = HashMap::new();
        // get every components' graphs
        self.components.iter().for_each(|component| {
            let _unique = components_reduced_graphs
                .insert(component.get_id(), component.get_reduced_graph().clone());
            debug_assert!(_unique.is_none())
        });
        // normalize components
        self.components
            .iter_mut()
            .for_each(|component| component.normal_form(&components_reduced_graphs, ctx));

        // normalize interface
        self.interface.normal_form(ctx);

        // Debug: test it is in normal form
        debug_assert!(self.is_normal_form());
    }

    /// Inline component application when it is needed.
    ///
    /// Inlining needed for "shifted causality loop".
    fn inline_when_needed(&mut self, ctx: &mut Ctx) {
        let components = self
            .components
            .iter()
            .map(|component| (component.get_id(), component.clone()))
            .collect::<HashMap<_, _>>();
        self.components
            .iter_mut()
            .for_each(|component| component.inline_when_needed(&components, ctx))
    }

    /// Schedule components' equations.
    pub fn schedule(&mut self) {
        self.components
            .iter_mut()
            .for_each(|component| component.schedule())
    }

    /// Generate dependency graphs for the interface.
    #[inline]
    fn generate_flows_dependency_graphs(&mut self) {
        self.interface.generate_flows_dependency_graphs()
    }

    /// Normalize [ir1] components in file.
    pub fn normalize(&mut self, ctx: &mut Ctx, errors: &mut Vec<Error>) -> TRes<()> {
        self.normal_form(ctx);
        self.generate_flows_dependency_graphs();
        self.no_unused(ctx).dewrap(errors)?;
        self.memorize(ctx).dewrap(errors)?;
        self.inline_when_needed(ctx);
        self.schedule();
        Ok(())
    }
}

pub mod dump_graph {
    prelude! {}
    use grust_compiler_common::json::{begin_json, end_json};

    impl File {
        /// Dump dependency graph with parallelization weights.
        pub fn dump_graph<P: AsRef<std::path::Path>>(&self, filepath: P, ctx: &Ctx) {
            begin_json(&filepath);
            self.components
                .iter()
                .for_each(|comp| comp.dump_graph(&filepath, ctx));
            end_json(&filepath);
        }
    }
}
