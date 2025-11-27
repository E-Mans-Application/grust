#![cfg_attr(feature = "diagnostics", feature(proc_macro_diagnostic))]

pub extern crate itertools;
pub extern crate json;
pub extern crate lazy_static;
pub extern crate once_cell;
pub extern crate petgraph;
pub extern crate proc_macro as macro1;
pub extern crate proc_macro2 as macro2;
pub extern crate quote;
pub extern crate rustc_hash;
pub extern crate serde;
pub extern crate strum;
pub extern crate syn;

#[macro_use]
pub mod prelude;

#[macro_use]
mod mk_new_def;

mod constant;
mod convert_case;
mod err;
mod ext;
mod stats;
// mod error;
pub mod check;
mod either;
mod hash_map;
mod location;
mod react_typ;
mod scope;
mod typ;

pub mod conf;
pub mod graph;
pub mod keyword;
pub mod op;
pub mod serialize;
pub mod synced;

pub mod import {
    pub use crate::prelude::*;
}
