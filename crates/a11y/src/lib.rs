//! AccessKit bridge for Sigil — projects the kernel node tree into
//! `accesskit::TreeUpdate`.
//!
//! This crate is the single consumer of every node's
//! [`sigil_kernel::A11yProps`]. Widgets author roles, names, state
//! signals, and live-region annotations on the kernel side; the
//! bridge here reads them and produces the AccessKit projection.
//!
//! See [DR-050] for the locked decisions.
//!
//! [DR-050]: ../../../docs/decisions/DR-050-g10-a11y.md

#![deny(missing_docs)]
// W5.2 — `sigil-reactive` is a dev-dep used only by `tests/reactive_flow.rs`.
// Phase 1.3.C — `sigil` is a dev-dep used only by `tests/live_regions.rs`.
// RC-focus finding 1 — `layout` is a dev-dep used only by
// `tests/bounds_projection.rs`. Cargo's lib-test target pulls all
// dev-deps into the lib compilation unit, which trips
// `unused_crate_dependencies` here. Aliases silence the lint without
// adding a real import.
#[cfg(test)]
use sigil as _;
#[cfg(test)]
use sigil_layout as _;
#[cfg(test)]
use sigil_reactive as _;

pub mod bridge;
pub mod mapping;
pub mod snapshot;

pub use bridge::{AccessKitBridge, BridgeError, RecordingBridge};
pub use mapping::{BoundsLookup, node, tree};
pub use snapshot::snapshot;
