//! AccessKit bridge for Sigil — projects the kernel node tree into
//! `accesskit::TreeUpdate`.
//!
//! This crate is the single consumer of every node's
//! [`sigil_kernel::A11yProps`]. Widgets author roles, names, state
//! signals, and live-region annotations on the kernel side; the
//! bridge here reads them and produces the AccessKit projection.
//!
//! See internal decision record DR-050 for the locked decisions.

#![deny(missing_docs)]
#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

pub mod bridge;
pub mod mapping;
pub mod snapshot;

pub use bridge::{AccessKitBridge, BridgeError, RecordingBridge};
pub use mapping::{BoundsLookup, node, tree};
pub use snapshot::snapshot;
