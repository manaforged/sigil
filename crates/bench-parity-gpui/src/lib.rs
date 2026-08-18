//! GPUI runner for the bench-parity harness.
//!
//! Timed surface matches Sigil for a given workload:
//! - `layout`: `AnyElement::layout_as_root` + `prepaint_at`
//!   (this is when `uniform_list` materialises the visible window)
//! - `layout_paint`: the same plus `paint` (caught if the test
//!   platform panics inside the scroll listener)
//!
//! `TestAppContext` is reused across samples when `reuse` is true.

#![allow(clippy::module_name_repetitions)]

use clap as _;
use serde as _;
use serde_json as _;
use time as _;

mod gpui_runner;

pub use gpui_runner::GpuiRunner;
