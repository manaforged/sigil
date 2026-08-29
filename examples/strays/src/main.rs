//! `strays` — every localhost server on this machine, and which ones nobody
//! is watching any more.
//!
//! A development session starts a server, ends, and leaves the port
//! held for hours. This lists every listening process, attributes it to the
//! source tree it runs in, and kills the ones that are past their limit.
//!
//! ```text
//! cargo run -p strays
//! ```
//!
//! A row is a stray when it runs inside a dev root, holds a listening port,
//! answers to no terminal, and has outlived the limit. All four conditions
//! matter: age and a missing terminal on their own describe every system
//! daemon on the machine.

#![allow(missing_docs)]

mod scan;
mod state;
mod view;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sigil::prelude::*;

use crate::state::AppState;

fn main() -> sigil::AppResult<()> {
    // Scan before the terminal comes up: the first read costs tens of
    // milliseconds, and doing it from the tree builder mounts the app and
    // then freezes it on its own first frame.
    let reader = Arc::new(Mutex::new(scan::Scanner::new()));
    let first = reader.lock().map(|mut r| r.read()).unwrap_or_default();
    sigil::run_tui(("Sigil - strays", 120, 32), move |cx: &mut Cx| {
        let state = AppState::adopt(
            Arc::clone(&reader),
            first.clone(),
            PathBuf::from(scan::DEV),
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
