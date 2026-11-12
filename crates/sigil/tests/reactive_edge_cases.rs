use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil::reactive::{Owner, batch, cleanup, effect, memo, signal};

#[path = "reactive_edge_cases/batch_update.rs"]
mod batch_update;
#[path = "reactive_edge_cases/graph.rs"]
mod graph;
#[path = "reactive_edge_cases/owners.rs"]
mod owners;
