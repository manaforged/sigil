#![allow(missing_docs)]

mod scan;
mod state;
mod view;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sigil::prelude::*;

use crate::state::AppState;

fn main() -> sigil::AppResult<()> {
    let reader = Arc::new(Mutex::new(scan::Scanner::new()));
    let first = reader.lock().map(|mut r| r.read()).unwrap_or_default();
    sigil::run_tui(("Sigil - strays", 120, 32), move |cx: &mut Cx| {
        let state = AppState::adopt(
            Arc::clone(&reader),
            first.clone(),
            scan::dev_root(),
            scan::LIMIT,
        );
        cx.add(view::app_tree(&state))
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::panic,
    reason = "demo harness; the invariant is proven by construction"
)]
#[path = "main_alignment_tests.rs"]
mod alignment;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::panic,
    reason = "demo harness; the invariant is proven by construction"
)]
#[path = "main_cost_tests.rs"]
mod cost;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::panic,
    reason = "demo harness; the invariant is proven by construction"
)]
#[path = "main_keys_tests.rs"]
mod keys;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::panic,
    reason = "demo harness; the invariant is proven by construction"
)]
#[path = "main_offload_tests.rs"]
mod offload;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::panic,
    reason = "demo harness; the invariant is proven by construction"
)]
#[path = "main_paint_tests.rs"]
mod paint;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::panic,
    reason = "demo harness; the invariant is proven by construction"
)]
#[path = "main_tests.rs"]
mod tests;
