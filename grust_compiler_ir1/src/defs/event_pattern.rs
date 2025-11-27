//! [EventPattern] module.

prelude! {}

#[derive(Debug, PartialEq, Clone)]
/// Event pattern kind.
pub enum Kind {
    /// Tuple pattern that matches tuples.
    Tuple {
        /// The elements of the tuple.
        elements: Vec<EventPattern>,
    },
    /// Event detection pattern.
    Let {
        /// The pattern receiving the value of the event.
        pattern: ir1::Pattern,
        /// The event to match.
        event_id: usize,
    },
    /// Rising edge.
    RisingEdge(stream::Expr),
}

mk_new! { impl Kind =>
    Tuple: tuple { elements: Vec<EventPattern> }
    Let: detect {
        pattern: ir1::Pattern,
        event_id: usize,
    }
    RisingEdge: rising_edge (expr: stream::Expr = expr)
}

/// EventPattern.
#[derive(Debug, PartialEq, Clone)]
pub struct EventPattern {
    /// EventPattern kind.
    pub kind: Kind,
    /// EventPattern type.
    pub typing: Option<Typ>,
    /// EventPattern location.
    pub loc: Loc,
}
impl HasLoc for EventPattern {
    fn loc(&self) -> Loc {
        self.loc
    }
}

impl EventPattern {
    /// Constructs pattern.
    ///
    /// Typing and location are empty.
    pub fn new(loc: impl Into<Loc>, kind: Kind) -> EventPattern {
        EventPattern {
            kind,
            typing: None,
            loc: loc.into(),
        }
    }

    /// Get pattern's type.
    pub fn get_typ(&self) -> Option<&Typ> {
        self.typing.as_ref()
    }
    /// Get pattern's mutable type.
    pub fn get_typ_mut(&mut self) -> Option<&mut Typ> {
        self.typing.as_mut()
    }
    /// Get pattern's identifiers.
    pub fn identifiers(&self) -> Vec<usize> {
        match &self.kind {
            Kind::RisingEdge(_) => vec![],
            Kind::Tuple { elements } => elements
                .iter()
                .flat_map(|pattern| pattern.identifiers())
                .collect(),
            Kind::Let { pattern, .. } => pattern.identifiers(),
        }
    }
    /// Get mutable references to pattern's identifiers.
    pub fn identifiers_mut(&mut self) -> Vec<&mut usize> {
        match &mut self.kind {
            Kind::RisingEdge(_) => vec![],
            Kind::Tuple { elements } => elements
                .iter_mut()
                .flat_map(|pattern| pattern.identifiers_mut())
                .collect(),
            Kind::Let { pattern, .. } => pattern.identifiers_mut(),
        }
    }

    /// Accumulates in `events_indices` the indices of events in the matched tuple.
    pub fn place_events(&self, events_indices: &mut HashMap<usize, usize>, idx: &mut usize) {
        match &self.kind {
            Kind::Tuple { elements } => elements
                .iter()
                .for_each(|pattern| pattern.place_events(events_indices, idx)),
            Kind::Let { event_id, .. } => {
                let _ = events_indices.entry(*event_id).or_insert_with(|| {
                    let v = *idx;
                    *idx += 1;
                    v
                });
            }
            Kind::RisingEdge(_) => (),
        }
    }

    /// Creates event tuple, stores the events and return rising_edges combined as a guard.
    pub fn create_tuple_pattern(
        self,
        tuple: &mut Vec<ir1::Pattern>,
        events_indices: &HashMap<usize, usize>,
    ) -> Option<ir1::stream::Expr> {
        match self.kind {
            Kind::Tuple { elements } => {
                let mut guard: Option<ir1::stream::Expr> = None;
                let mut combine_guard = |opt_guard: Option<ir1::stream::Expr>| {
                    if let Some(add_guard) = opt_guard {
                        if let Some(old_guard) = guard.take() {
                            guard = Some(ir1::stream::Expr::new(
                                old_guard.loc(),
                                ir1::stream::Kind::expr(ir1::expr::Kind::binop(
                                    BOp::And,
                                    old_guard,
                                    add_guard,
                                )),
                            ));
                        } else {
                            guard = Some(add_guard);
                        }
                    }
                };

                elements.into_iter().for_each(|pattern| {
                    let opt_guard = pattern.create_tuple_pattern(tuple, events_indices);
                    // combine all rising edge detections
                    combine_guard(opt_guard);
                });

                guard
            }
            Kind::Let {
                pattern: matching_pat,
                event_id,
            } => {
                let event_pattern = ir1::pattern::Pattern::new(
                    self.loc,
                    ir1::pattern::Kind::present(event_id, matching_pat),
                );

                // put event in tuple
                let idx = events_indices[&event_id];
                *tuple.get_mut(idx).unwrap() = event_pattern;

                None
            }
            Kind::RisingEdge(expr) => {
                let loc = expr.loc();
                let guard = ir1::stream::Kind::rising_edge(expr);
                Some(ir1::stream::Expr::new(loc, guard))
            }
        }
    }
}
