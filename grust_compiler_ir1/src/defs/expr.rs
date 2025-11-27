//! [Expr] module.

prelude!(graph::Label);

pub type Arm<E> = (Pattern, Option<E>, Vec<Stmt<E>>, E);

#[derive(Debug, PartialEq, Clone)]
/// expression kind.
pub enum Kind<E> {
    /// Constant expression.
    Constant {
        /// The constant.
        constant: Constant,
    },
    /// Identifier expression.
    Identifier {
        /// Element identifier.
        id: usize,
    },
    /// UnOp expression.
    UnOp {
        /// The unary operator.
        op: UOp,
        /// The input expression.
        expr: Box<E>,
    },
    /// BinOp expression.
    BinOp {
        /// The unary operator.
        op: BOp,
        /// The left expression.
        lft: Box<E>,
        /// The right expression.
        rgt: Box<E>,
    },
    /// IfThenElse expression.
    IfThenElse {
        /// Condition.
        cnd: Box<E>,
        /// `then` branch.
        thn: Box<E>,
        /// `else` branch.
        els: Box<E>,
    },
    /// Application expression.
    Application {
        /// The expression applied.
        fun: Box<E>,
        /// The inputs to the expression.
        inputs: Vec<E>,
    },
    /// Lambda expression.
    Lambda {
        /// The inputs to the lambda.
        inputs: Vec<usize>,
        /// The expression abstracted.
        expr: Box<Expr>,
        /// To make Rust type check.
        _phantom: std::marker::PhantomData<E>,
    },
    /// Structure expression.
    Structure {
        /// The structure id.
        id: usize,
        /// The fields associated with their expressions.
        fields: Vec<(usize, E)>,
    },
    /// Enumeration expression.
    Enumeration {
        /// The enumeration id.
        enum_id: usize,
        /// The element id.
        elem_id: usize,
    },
    /// Array expression.
    Array {
        /// The elements inside the array.
        elements: Vec<E>,
    },
    /// Tuple expression.
    Tuple {
        /// The elements.
        elements: Vec<E>,
    },
    /// Pattern matching expression.
    MatchExpr {
        /// The expression to match.
        expr: Box<E>,
        /// The different matching cases.
        arms: Vec<Arm<E>>,
    },
    /// Field access expression.
    FieldAccess {
        /// The structure expression.
        expr: Box<E>,
        /// The field to access.
        field: Ident, // can not be a usize because we don't know the structure type
    },
    /// Tuple element access expression.
    TupleElementAccess {
        /// The tuple expression.
        expr: Box<E>,
        /// The element to access.
        element_number: usize,
    },
    /// Array element access expression.
    ArrayAccess {
        /// The tuple expression.
        expr: Box<E>,
        /// The index to access.
        index: syn::LitInt,
    },
    /// Array map operator expression.
    Map {
        /// The array expression.
        expr: Box<E>,
        /// The function expression.
        fun: Box<E>,
    },
    /// Array fold operator expression.
    Fold {
        /// The array expression.
        array: Box<E>,
        /// The initialization expression.
        init: Box<E>,
        /// The function expression.
        fun: Box<E>,
    },
    /// Array sort operator expression.
    Sort {
        /// The array expression.
        expr: Box<E>,
        /// The function expression.
        fun: Box<E>,
    },
    /// Arrays zip operator expression.
    Zip {
        /// The array expressions.
        arrays: Vec<E>,
    },
}

mk_new! { impl{E} Kind<E> =>
    Constant: constant { constant: Constant }
    Identifier: ident { id : usize }
    UnOp: unop {
        op: UOp,
        expr: E = expr.into(),
    }
    BinOp: binop {
        op: BOp,
        lft: E = lft.into(),
        rgt: E = rgt.into(),
    }
    IfThenElse: if_then_else {
        cnd: E = cnd.into(),
        thn: E = thn.into(),
        els: E = els.into(),
    }
    Application: app {
        fun: E = fun.into(),
        inputs: Vec<E>,
    }
    Lambda: lambda {
        inputs: Vec<usize>,
        expr: Expr = expr.into(),
        _phantom = std::marker::PhantomData,
    }
    Structure: structure {
        id: usize,
        fields: Vec<(usize, E)>,
    }
    Enumeration: enumeration {
        enum_id: usize,
        elem_id: usize,
    }
    Array: array { elements: Vec<E> }
    Tuple: tuple { elements: Vec<E> }
    MatchExpr: match_expr {
        expr: E = expr.into(),
        arms: Vec<Arm<E>>,
    }
    FieldAccess: field_access {
        expr: E = expr.into(),
        field: impl Into<Ident> = field.into(),
    }
    TupleElementAccess: tuple_access {
        expr: E = expr.into(),
        element_number: usize,
    }
    ArrayAccess: array_access {
        expr: E = expr.into(),
        index: syn::LitInt,
    }
    Map: map {
        expr: E = expr.into(),
        fun: E = fun.into(),
    }
    Fold: fold {
        array: E = array.into(),
        init: E = init.into(),
        fun: E = fun.into(),
    }
    Sort: sort{
        expr: E = expr.into(),
        fun: E = fun.into(),
    }
    Zip: zip { arrays: Vec<E> }
}

impl<E: HasWeight> HasWeight for Kind<E>
where
    Stmt<E>: HasWeight,
{
    fn weight(&self, wb: &synced::WeightBounds, ctx: &Ctx) -> synced::Weight {
        use synced::weight;
        use Kind::*;

        match self {
            Constant { .. } => weight::zero,
            Identifier { id } => ctx.get_weight_percent_hint(*id).unwrap_or(weight::lo),
            UnOp { expr, .. } => expr.weight(wb, ctx) + weight::lo,
            BinOp { lft, rgt, .. } => lft.weight(wb, ctx) + rgt.weight(wb, ctx) + weight::lo,
            IfThenElse { cnd, thn, els } => {
                cnd.weight(wb, ctx) + thn.weight(wb, ctx).max(els.weight(wb, ctx)) + weight::lo
            }
            Application { fun, inputs } => {
                fun.weight(wb, ctx) + w8!(wb, ctx => sum inputs) + weight::hi
            }
            Lambda { expr, .. } => expr.weight(wb, ctx),
            Structure { fields, .. } => w8!(sum fields, |(_, e)| e.weight(wb, ctx)) + weight::lo,
            Enumeration { .. } => weight::zero,
            Array { elements } | Tuple { elements } => w8!(wb, ctx => sum elements) + weight::mid,
            MatchExpr { expr, arms } => {
                expr.weight(wb, ctx)
                    + arms
                        .iter()
                        .map(|(_, expr_opt, stmts, expr)| {
                            w8!(wb, ctx => weight? expr_opt.as_ref())
                                + w8!(wb, ctx => sum stmts)
                                + expr.weight(wb, ctx)
                        })
                        .max()
                        .unwrap_or(weight::zero)
            }
            FieldAccess { expr, .. }
            | TupleElementAccess { expr, .. }
            | ArrayAccess { expr, .. } => expr.weight(wb, ctx),
            Map { expr, fun } => {
                // well, we can't do much without knowing the length of the array...
                expr.weight(wb, ctx) + fun.weight(wb, ctx) + weight::hi
            }
            Fold { array, init, fun } => {
                // still want to know the length of the array...
                array.weight(wb, ctx) + fun.weight(wb, ctx) + init.weight(wb, ctx) + weight::hi
            }
            Sort { expr, fun } => {
                // length of the array™, but sorting is expensive anyway
                expr.weight(wb, ctx) * fun.weight(wb, ctx) + weight::hi
            }
            Zip { arrays } => {
                // #lengthofthearrays
                w8!(wb, ctx => sum arrays)
            }
        }
    }
}

impl<E> Kind<E> {
    /// Propagate a predicate over the expression tree.
    pub fn propagate_predicate<F1, F2>(&self, expr_pred: F1, stmt_pred: F2) -> bool
    where
        F1: Fn(&E) -> bool,
        F2: Fn(&ir1::Stmt<E>) -> bool,
    {
        match self {
            Kind::Constant { .. }
            | Kind::Identifier { .. }
            | Kind::Lambda { .. }
            | Kind::Enumeration { .. } => true,
            Kind::UnOp { expr, .. } => expr_pred(expr),
            Kind::BinOp { lft, rgt, .. } => expr_pred(lft) && expr_pred(rgt),
            Kind::IfThenElse { cnd, thn, els } => {
                expr_pred(cnd) && expr_pred(thn) && expr_pred(els)
            }
            Kind::Application { fun, inputs } => expr_pred(fun) && inputs.iter().all(&expr_pred),
            Kind::Structure { fields, .. } => {
                fields.iter().all(|(_, expression)| expr_pred(expression))
            }
            Kind::Array { elements } | Kind::Tuple { elements } => elements.iter().all(&expr_pred),
            Kind::MatchExpr { expr, arms } => {
                expr_pred(expr)
                    && arms.iter().all(|(_, option, body, expr)| {
                        body.iter().all(&stmt_pred)
                            && option.as_ref().is_none_or(&expr_pred)
                            && expr_pred(expr)
                    })
            }
            Kind::FieldAccess { expr, .. }
            | Kind::TupleElementAccess { expr, .. }
            | Kind::ArrayAccess { expr, .. } => expr_pred(expr),
            Kind::Map { expr, fun } => expr_pred(expr) && expr_pred(fun),
            Kind::Fold { array, init, fun } => {
                expr_pred(array) && expr_pred(init) && expr_pred(fun)
            }
            Kind::Sort { expr, fun } => expr_pred(expr) && expr_pred(fun),
            Kind::Zip { arrays } => arrays.iter().all(expr_pred),
        }
    }

    pub fn is_default_constant(&self) -> bool {
        if let Self::Constant { constant } = self {
            constant.is_default()
        } else {
            false
        }
    }
}

/// expression.
#[derive(Debug, PartialEq, Clone)]
pub struct Expr {
    /// Expression kind.
    pub kind: ir1::expr::Kind<Self>,
    /// Expression type.
    pub typing: Option<Typ>,
    /// Expression location.
    pub loc: Loc,
    /// Expression dependencies.
    pub dependencies: ir1::Dependencies,
}

impl HasWeight for Expr {
    fn weight(&self, bounds: &synced::WeightBounds, ctx: &Ctx) -> synced::Weight {
        self.kind.weight(bounds, ctx)
    }
}

impl Expr {
    pub fn new(kind: ir1::expr::Kind<Self>, loc: Loc) -> Self {
        Expr {
            kind,
            typing: None,
            loc,
            dependencies: Dependencies::new(),
        }
    }
    /// Get expression's type.
    pub fn get_type(&self) -> Option<&Typ> {
        self.typing.as_ref()
    }
    /// Get expression's mutable type.
    pub fn get_type_mut(&mut self) -> Option<&mut Typ> {
        self.typing.as_mut()
    }
    /// Get expression's dependencies.
    pub fn get_dependencies(&self) -> &Vec<(usize, Label)> {
        self.dependencies
            .get()
            .expect("internal error: there should be dependencies")
    }
}
