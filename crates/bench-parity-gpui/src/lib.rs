//! GPUI framework runner for the sigil-bench-parity harness.
//!
//! # Measurement layer
//!
//! GPUI's headless path on Windows is `TestAppContext` + `VisualTestContext`
//! (under the `test-support` feature). `VisualTestContext::draw` runs the
//! full frame — `request_layout` → `prepaint` → `paint`. There is no
//! public API to isolate the layout pass alone, so this runner measures
//! **Layer 2 minus paint scene emission**:
//!
//! - `request_layout` runs (the bench element's first sizing pass).
//! - `prepaint` runs (resolves visible window, calls the render-range
//!   closure with the actual `Range<usize>` that would be materialised).
//! - `paint` **panics** in `Interactivity::paint_scroll_listener` because
//!   `VisualTestContext::draw` does not push a "view" onto
//!   `rendered_entity_stack` the way a real `Render` impl does. The panic
//!   is caught with [`std::panic::catch_unwind`]; its unwinding cost is
//!   constant across iterations and dominates none of the work.
//!
//! Sigil's reference runner measures `LayoutEngine::compute` only
//! (Layer 1). The two measurements are **NOT identical layers**. Any
//! cross-framework comparison must surface that difference; the
//! aggregator emits the rows verbatim and leaves the comparative call
//! to a human-curated analysis doc.
//!
//! # Equivalence check
//!
//! `visible_window_indices` is computed by capturing every range the
//! GPUI render-range closure is invoked with. The first one or two
//! calls are GPUI's internal row-size probes (`0..1`); the last call
//! whose `range.len() > 1` is the genuine visible window. We compare
//! its length against the workload's expected `ceil(viewport_h / row_h) + 1`.

#![allow(clippy::module_name_repetitions)]

use clap as _;
use serde as _;
use serde_json as _;
use time as _;

mod gpui_runner;

pub use gpui_runner::GpuiRunner;
