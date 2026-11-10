pub(crate) mod actions;
#[cfg(any(test, feature = "kittest", feature = "desktop"))]
pub(crate) mod bridge;
mod links;
pub(crate) mod mapping;
mod modal_scope;
pub(crate) mod snapshot;
mod text;

#[cfg(all(feature = "desktop", not(feature = "kittest")))]
pub(crate) use bridge::AccessKitBridge;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use bridge::AccessKitBridge;
#[cfg(all(feature = "desktop", not(feature = "kittest")))]
pub(crate) use mapping::tree;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use mapping::{BoundsLookup, node, tree};
pub use snapshot::snapshot;
