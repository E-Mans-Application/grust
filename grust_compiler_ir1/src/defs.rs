pub(crate) mod component;
pub(crate) mod dependencies;
pub(crate) mod file;
pub(crate) mod function;
pub(crate) mod identifier_creator;
pub(crate) mod once_cell;

pub mod contract;
pub mod ctx;
pub mod event_pattern;
pub mod expr;
pub mod memory;
pub mod pattern;
pub mod stmt;
pub mod stream;
pub mod typedef;

pub mod flow;

pub mod interface;

prelude! {}

pub fn raw_from_ast(ast: Ast, symbols: &mut Ctx, errors: &mut Vec<Error>) -> TRes<File> {
    let mut ctx = ctx::Simple::new(symbols, errors);
    ast.into_ir1(&mut ctx)
}

pub fn from_ast_timed(
    ast: Ast,
    symbols: &mut Ctx,
    mut stats: StatsMut,
) -> Result<File, Vec<Error>> {
    let mut errors_vec = vec![];
    let errors = &mut errors_vec;
    macro_rules! check_errors {
        {} => {
            if !errors.is_empty() { return Err(errors_vec); }
        };
        { $desc:expr, $e:expr $(,)? } => {{
            check_errors!();
            match stats.timed_with($desc, $e) {
                Ok(res) => res,
                Err(ErrorDetected) => {
                    if errors.is_empty() {
                        panic!("empty errors :/ ({})", $desc);
                    }
                    for e in errors {
                        e.add_note_mut(Note::new(None, concat!("during ", $desc)))
                    }
                    return Err(errors_vec);
                }
            }
        }};
    }
    let mut ir1 = check_errors!("parsing (ir0 → ir1)", |_| raw_from_ast(
        ast, symbols, errors
    ));
    check_errors!("type-checking (ir1)", |_| ir1.typ_check(symbols, errors));
    check_errors!("dependency graph generation (ir1)", |sub_stats| ir1
        .generate_dependency_graphs(symbols, sub_stats, errors),);
    check_errors!("causality analysis (ir1)", |_| ir1
        .causality_analysis(symbols, errors));
    check_errors!("reactive-type-checking (ir1)", |_| ir1
        .react_typ_check(symbols, errors));
    check_errors!("normalization (ir1)", |_| ir1.normalize(symbols, errors));
    Ok(ir1)
}

pub fn from_ast(ast: Ast, symbols: &mut Ctx) -> Result<File, Vec<Error>> {
    from_ast_timed(ast, symbols, Stats::new().as_mut())
}
