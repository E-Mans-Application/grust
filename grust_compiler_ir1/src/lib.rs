#![allow(missing_docs)]
#[macro_use]
pub mod prelude;

mod defs;

pub use defs::*;

mod dependencies;
mod ir0_into_ir1;
mod ir0_store;
mod reactive_typing;
mod typing;
mod unused;

pub mod import {
    pub use crate::prelude::ir1::{self, Ir0IntoIr1, Ir0Store, Typing};
}
