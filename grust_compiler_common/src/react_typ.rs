prelude! {}

/// GRust reactive types.
///
/// [ReactTyp] enumeration used for the reactive typing of a GRust program.
///
/// It represents if the associated element is continuous (depending on `time()` or untriggered
/// event emissions and `last` memories) or discrete:
///
/// - `x` is [ReactTyp::Continuous] denotes that `x` depends on those constructs (`x` is a
///   continuous flow),
/// - `x` is [ReactTyp::Discrete] denotes that `x` has no such dependencies (`x` is a discrete flow).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReactTyp {
    /// Depending on `time()` or untriggered event emissions and `last` memories.
    Continuous,
    /// No dependencies on `time()` or untriggered event emissions and `last` memories.
    Discrete,
}

mk_new! { impl ReactTyp =>
    Continuous: c()
    Discrete: d()
}

impl ReactTyp {
    /// Check if `self` matches the expected [ReactTyp].
    pub fn check_eq(&self, expected: ReactTyp, loc: Loc) -> URes {
        check::react_typ::expect(loc, *self, expected)
    }
}

impl Display for ReactTyp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReactTyp::Continuous => write!(f, "continuous"),
            ReactTyp::Discrete => write!(f, "discrete"),
        }
    }
}
