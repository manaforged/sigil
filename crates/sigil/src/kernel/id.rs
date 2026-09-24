use slotmap::{DefaultKey, Key, KeyData};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(pub(crate) DefaultKey);

impl NodeId {
    #[must_use]
    pub fn to_accesskit(self) -> accesskit::NodeId {
        accesskit::NodeId(self.0.data().as_ffi())
    }

    #[must_use]
    pub fn from_accesskit(id: accesskit::NodeId) -> Self {
        Self(DefaultKey::from(KeyData::from_ffi(id.0)))
    }

    #[must_use]
    pub(crate) const fn key(self) -> DefaultKey {
        self.0
    }

    #[must_use]
    pub(crate) const fn from_key(key: DefaultKey) -> Self {
        Self(key)
    }
}
