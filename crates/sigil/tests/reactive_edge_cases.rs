#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::reactive::{Owner, Runtime, batch, effect, on_cleanup, signal};

#[path = "reactive_edge_cases/batch_update.rs"]
mod batch_update;
#[path = "reactive_edge_cases/graph.rs"]
mod graph;
#[path = "reactive_edge_cases/owners.rs"]
mod owners;
