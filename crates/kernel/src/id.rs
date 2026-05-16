//! Stable node identity.

use slotmap::{DefaultKey, Key};

/// Stable, generational identity for a node in a [`Cx`](crate::Cx).
///
/// The backing key is a `slotmap` key. Removing a node and reusing
/// its arena slot bumps the generation, so stale node IDs do not
/// alias newly-created nodes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub(crate) DefaultKey);

impl NodeId {
    /// Convert this ID into AccessKit's `u64`-backed node identity.
    ///
    /// The `slotmap` FFI value packs index and generation, matching
    /// AccessKit's requirement that one logical element keeps the same
    /// identity across updates while removed/reused slots never alias.
    #[must_use]
    pub fn to_accesskit(self) -> accesskit::NodeId {
        accesskit::NodeId(self.0.data().as_ffi())
    }

    pub(crate) const fn key(self) -> DefaultKey {
        self.0
    }
}
