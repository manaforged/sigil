#[cfg(test)]
#[path = "a11y/lib_tests.rs"]
mod tests;

pub(crate) mod actions;
pub mod bridge;
pub mod mapping;
pub mod snapshot;
mod text;

pub use bridge::{AccessKitBridge, BridgeError, RecordingBridge};
pub use mapping::{BoundsLookup, node, tree};
pub use snapshot::snapshot;
