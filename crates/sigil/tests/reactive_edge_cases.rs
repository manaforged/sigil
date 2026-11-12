use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil::reactive::{Owner, batch, effect, memo, on_cleanup, signal};

#[path = "reactive_edge_cases/batch_update.rs"]
mod batch_update;
#[path = "reactive_edge_cases/graph.rs"]
mod graph;
#[path = "reactive_edge_cases/owners.rs"]
mod owners;
