pub(crate) mod actions;
#[cfg(window_host)]
pub(crate) mod bridge;
mod links;
pub(crate) mod mapping;
mod modal_scope;
pub(crate) mod snapshot;
mod text;

#[cfg(live_desktop)]
pub(crate) use bridge::AccessKitBridge;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use bridge::AccessKitBridge;
#[cfg(live_desktop)]
pub(crate) use mapping::tree;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use mapping::{BoundsLookup, node, tree};
pub use snapshot::snapshot;
