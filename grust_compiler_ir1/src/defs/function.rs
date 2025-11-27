//! [Function] module.

prelude! {}

#[derive(Debug, PartialEq)]
/// GRust function AST.
pub struct FunctionBody {
    /// Function's contract.
    pub contract: ir1::Contract,
    /// Function's statements.
    pub statements: Vec<ir1::Stmt<ir1::Expr>>,
    /// Logs
    pub logs: Vec<usize>,
    /// Function's returned expression and its type.
    pub returned: ir1::Expr,
}

#[derive(Debug, PartialEq)]
/// GRust function AST.
pub struct Function {
    /// Function identifier.
    pub id: usize,
    /// Function's
    pub body_or_path: Either<FunctionBody, syn::Path>,
    /// Function location.
    pub loc: Loc,
}

impl Function {
    pub fn new(
        id: usize,
        contract: ir1::Contract,
        statements: Vec<ir1::Stmt<ir1::Expr>>,
        logs: Vec<usize>,
        returned: ir1::Expr,
        loc: Loc,
    ) -> Self {
        Self {
            id,
            body_or_path: Either::Left(FunctionBody {
                contract,
                statements,
                logs,
                returned,
            }),
            loc,
        }
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn new_ext(id: usize, path: syn::Path, loc: Loc) -> Self {
        Self {
            id,
            body_or_path: Either::Right(path),
            loc,
        }
    }

    pub fn body_ref(&self) -> Option<&FunctionBody> {
        self.body_or_path.as_ref().left()
    }

    pub fn body_mut(&mut self) -> Option<&mut FunctionBody> {
        self.body_or_path.as_mut().left()
    }
}
