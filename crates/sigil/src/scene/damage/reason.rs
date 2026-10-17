#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FullReason {
    #[cfg(feature = "scene")]
    FirstFrame,
    #[cfg(feature = "scene")]
    Untracked,
    NoBackbuffer,
    #[cfg(feature = "scene")]
    TranslucentClear,
    #[cfg(feature = "scene")]
    ViewportResized,
    #[cfg(feature = "scene")]
    KeyChanged,
    NonLocalLayerEffect,
    ChangedShare,
    UnboundedChange,
    #[cfg(feature = "scene")]
    RegionShare,
}

#[cfg(feature = "scene")]
impl FullReason {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::FirstFrame => "first frame",
            Self::Untracked => "untracked target",
            Self::NoBackbuffer => "no back buffer",
            Self::TranslucentClear => "translucent clear",
            Self::ViewportResized => "viewport resize",
            Self::KeyChanged => "frame key changed",
            Self::NonLocalLayerEffect => "non-local layer effect",
            Self::ChangedShare => "changed share over threshold",
            Self::UnboundedChange => "unbounded change",
            Self::RegionShare => "damage area over threshold",
        }
    }
}
