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

pub mod bridge;
pub mod mapping;
pub mod snapshot;

pub use bridge::{BridgeError, RecordingBridge};
pub use mapping::{node, tree};
pub use snapshot::snapshot;
