//! Basic types, helpers and re-exports.

/// Imports the compiler prelude.
#[macro_export]
macro_rules! prelude {
    { just $($imports:tt)* } => {
        use $crate::prelude::{$($imports)*};
    };
    { $($imports:tt)* } => {
        use $crate::prelude::{*, $($imports)*};
    };
}

pub use grust_compiler_common::import::*;
pub use grust_compiler_ir0::import::*;

pub mod ir1 {
    pub use crate::{
        defs::{
            component::{Component, ComponentBody, ComponentSignature},
            contract::{self, Contract},
            ctx,
            dependencies::Dependencies,
            event_pattern::{self, EventPattern},
            expr::{self, Expr},
            file::File,
            flow, from_ast, from_ast_timed,
            function::Function,
            identifier_creator::IdentifierCreator,
            interface::{self, Interface, Service},
            memory::{self, Memory},
            once_cell::OnceCell,
            pattern::{self, Pattern},
            stmt::{self, Stmt},
            stream,
            typedef::{self, Typedef},
        },
        dependencies::DepCtx,
        ir0_into_ir1::Ir0IntoIr1,
        ir0_store::{
            Ir0Store, Ir0StoreEventPattern, Ir0StoreIdents, Ir0StoreInit, Ir0StorePattern,
            Ir0StoreStmtPattern,
        },
        typing::Typing,
        unused::Unused,
    };
}

pub use ir1::*;
