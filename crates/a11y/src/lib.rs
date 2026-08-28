#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

pub mod bridge;
pub mod mapping;
pub mod snapshot;

pub use bridge::{AccessKitBridge, BridgeError, RecordingBridge};
pub use mapping::{BoundsLookup, node, tree};
pub use snapshot::snapshot;
