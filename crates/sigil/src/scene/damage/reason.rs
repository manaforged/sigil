#![cfg_attr(
    not(feature = "scene"),
    allow(
        dead_code,
        reason = "the terminal uses the ink bounds; the GPU renderer uses the rest"
    )
)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FullReason {
    FirstFrame,
    Untracked,
    NoBackbuffer,
    TranslucentClear,
    ViewportResized,
    KeyChanged,
    NonLocalLayerEffect,
    ChangedShare,
    UnboundedChange,
    RegionShare,
}

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
