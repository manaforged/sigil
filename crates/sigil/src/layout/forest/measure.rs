#![allow(clippy::redundant_pub_crate)]

use super::text_memo::{text_memo_get, text_memo_key, text_memo_store};
use super::{ComputedBox, LayoutForest, LocalId, NodeRef};
use crate::layout::TextMeasure;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SpaceConstraint {
    Definite(f32),
    MinContent,
    MaxContent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AvailableSpace {
    pub(crate) w: SpaceConstraint,
    pub(crate) h: SpaceConstraint,
}

impl AvailableSpace {
    #[must_use]
    pub(crate) const fn definite(w: f32, h: f32) -> Self {
        Self {
            w: SpaceConstraint::Definite(w),
            h: SpaceConstraint::Definite(h),
        }
    }
}

pub(crate) struct MeasureScope<'a> {
    forest: &'a LayoutForest,
    text: &'a dyn TextMeasure,
    available: AvailableSpace,
}

impl<'a> MeasureScope<'a> {
    pub(crate) const fn new(
        forest: &'a LayoutForest,
        text: &'a dyn TextMeasure,
        available: AvailableSpace,
    ) -> Self {
        Self {
            forest,
            text,
            available,
        }
    }

    pub(crate) const fn forest(&self) -> &LayoutForest {
        self.forest
    }

    pub(crate) const fn text(&self) -> &dyn TextMeasure {
        self.text
    }

    pub(crate) const fn available(&self) -> AvailableSpace {
        self.available
    }

    pub(crate) fn write(&self, id: LocalId, value: ComputedBox) {
        let idx = id.index as usize;
        let Some(entry) = self.forest.nodes.get(idx) else {
            return;
        };
        if !entry.alive || entry.tag != id.tag {
            return;
        }
        if let Some(slot) = self.forest.boxes.get(idx) {
            slot.write(value, self.forest.current_frame());
        }
    }

    pub(crate) fn write_node(&self, node: &NodeRef<'_>, value: ComputedBox) {
        self.write(node.id(), value);
    }

    pub(crate) fn measure_text(
        &self,
        node: NodeRef<'_>,
        known: crate::layout::style_data::Size<Option<f32>>,
        wrap_to: Option<f32>,
    ) -> Option<crate::layout::style_data::Size<f32>> {
        let text = node.text()?;
        let memo_key = text_memo_key(node.id(), known, wrap_to, text);
        if let Some(hit) = text_memo_get(memo_key) {
            return Some(hit);
        }
        let width = if text.wrap {
            known.width.or(wrap_to).map_or_else(
                || space_constraint_to_hal_dim(self.available.w),
                crate::hal::Dim::Fixed,
            )
        } else {
            crate::hal::Dim::MaxContent
        };
        let space = crate::hal::Size {
            width,
            height: space_constraint_to_hal_dim(self.available.h),
        };
        let measured = text.style.as_ref().map_or_else(
            || self.text.measure(&text.content, space),
            |style| self.text.measure_styled(&text.content, style, space),
        );
        let natural_w = if text.ellipsis {
            known
                .width
                .or(wrap_to)
                .map_or(measured.width, |box_w| measured.width.min(box_w))
        } else {
            measured.width
        };
        let out = crate::layout::style_data::Size {
            width: known.width.unwrap_or(natural_w),
            height: known.height.unwrap_or(measured.height),
        };
        text_memo_store(memo_key, out);
        Some(out)
    }
}

const fn space_constraint_to_hal_dim(space: SpaceConstraint) -> crate::hal::Dim {
    match space {
        SpaceConstraint::Definite(value) => crate::hal::Dim::Fixed(value),
        SpaceConstraint::MinContent => crate::hal::Dim::MinContent,
        SpaceConstraint::MaxContent => crate::hal::Dim::MaxContent,
    }
}
