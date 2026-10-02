use super::Plan;
use crate::profile::{Counter, Hotspot, attribute, count};

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

pub(crate) fn note_plan(plan: &Plan) {
    match plan {
        Plan::Unchanged => count(Counter::GpuPlanUnchanged),
        Plan::Regions(_) => count(Counter::GpuPlanRegions),
        Plan::Full(reason) => {
            count(Counter::GpuPlanFull);
            attribute(Hotspot::GpuFullRepaint, 1, || reason.name().to_owned());
        }
    }
}
