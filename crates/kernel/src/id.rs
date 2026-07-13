//! Stable node identity.

use slotmap::{DefaultKey, Key, KeyData};

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

    /// Reconstruct a `NodeId` from AccessKit's `u64`-backed node identity.
    ///
    /// Inverse of [`Self::to_accesskit`]. Used when an inbound
    /// `accesskit::ActionRequest::target_node` needs to be resolved back
    /// to the kernel node that authored it (W5-A11Y, DR-050 §4).
    #[must_use]
    pub fn from_accesskit(id: accesskit::NodeId) -> Self {
        Self(DefaultKey::from(KeyData::from_ffi(id.0)))
    }

    /// Slotmap key backing this node identity.
    ///
    /// Exposed for downstream crates (e.g. `sigil-layout`) that mirror the
    /// kernel's slotmap and want `slotmap::SecondaryMap` direct-indexed
    /// lookup instead of hashing.
    #[must_use]
    pub const fn key(self) -> DefaultKey {
        self.0
    }

    /// Reconstruct a `NodeId` from a previously-extracted slotmap key.
    ///
    /// Pair with [`Self::key`] to round-trip identities through a
    /// `slotmap::SecondaryMap` keyed map without going through hashing.
    #[must_use]
    pub const fn from_key(key: DefaultKey) -> Self {
        Self(key)
    }
}
