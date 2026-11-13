#![allow(clippy::redundant_pub_crate)]

use super::text_memo::{min_content_memo_key, text_memo_key};
use super::{ComputedBox, LayoutForest, LocalId, NodeRef};
use crate::layout::TextMeasure;
use crate::layout::resolve::NodeEdges;

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
    #[cfg(feature = "kittest")]
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

    pub(crate) fn font_generation(&self) -> u64 {
        self.forest
            .fonts
            .get()
            .unwrap_or_else(|| self.text.font_generation())
    }

    pub(crate) const fn available(&self) -> AvailableSpace {
        self.available
    }

    pub(crate) fn write(&self, id: LocalId, value: ComputedBox) {
        if let Some(cell) = self
            .forest
            .node(id)
            .and_then(|_| self.forest.boxes.get(id.index as usize))
        {
            cell.write(value, self.forest.current_frame());
        }
    }

    pub(crate) fn write_node(&self, node: &NodeRef<'_>, value: ComputedBox) {
        self.write(node.id(), value);
    }

    pub(crate) fn measure_leaf_text(
        &self,
        node: NodeRef<'_>,
        border_known: crate::layout::style_data::Axes<Option<f32>>,
        cb_width: f32,
        edges: &NodeEdges,
    ) -> Option<crate::layout::style_data::Axes<f32>> {
        let inline = edges.inline_chrome();
        let block = edges.block_chrome();
        let known = crate::layout::style_data::Axes {
            width: border_known.width.map(|w| (w - inline).max(0.0)),
            height: border_known.height.map(|h| (h - block).max(0.0)),
        };
        self.measure_text(node, known, Some((cb_width - inline).max(0.0)))
    }

    pub(crate) fn min_content_width(&self, node: NodeRef<'_>) -> Option<f32> {
        let text = node.text()?;
        if text.ellipsis {
            return None;
        }
        let memo_key = min_content_memo_key(node.id(), text, self.font_generation());
        if let Some(hit) = self.forest.text_memo.borrow_mut().get(memo_key) {
            return Some(hit.width);
        }
        let width = if text.wrap {
            crate::geometry::Dim::MinContent
        } else {
            crate::geometry::Dim::MaxContent
        };
        let measured = self.measure_at(text, width);
        let out = crate::layout::style_data::Axes {
            width: measured.width,
            height: measured.height,
        };
        self.forest.text_memo.borrow_mut().store(memo_key, out);
        Some(measured.width)
    }

    fn measure_text(
        &self,
        node: NodeRef<'_>,
        known: crate::layout::style_data::Axes<Option<f32>>,
        wrap_to: Option<f32>,
    ) -> Option<crate::layout::style_data::Axes<f32>> {
        let text = node.text()?;
        let mut memo_key = text_memo_key(node.id(), known, wrap_to, text, self.font_generation());
        if text.wrap && known.width.is_none() && wrap_to.is_none() {
            match self.available.w {
                SpaceConstraint::Definite(value) => memo_key.wrap = value.to_bits(),
                SpaceConstraint::MinContent => memo_key.min_content = true,
                SpaceConstraint::MaxContent => {}
            }
        }
        let last = self.forest.last_text.get(node.id().index as usize);
        if let Some((_, size)) = last
            .and_then(std::cell::Cell::get)
            .filter(|(key, _)| *key == memo_key)
        {
            return Some(size);
        }
        if let Some(hit) = self.forest.text_memo.borrow_mut().get(memo_key) {
            if let Some(last) = last {
                last.set(Some((memo_key, hit)));
            }
            return Some(hit);
        }
        let out = self.shape(text, known, wrap_to);
        self.forest.text_memo.borrow_mut().store(memo_key, out);
        if let Some(last) = last {
            last.set(Some((memo_key, out)));
        }
        Some(out)
    }

    fn measure_at(
        &self,
        text: &super::LayoutText,
        width: crate::geometry::Dim,
    ) -> crate::geometry::Size {
        let space = crate::geometry::Constraint {
            width,
            height: crate::geometry::Dim::MaxContent,
        };
        text.style.as_ref().map_or_else(
            || self.text.measure(&text.content, space),
            |style| self.text.measure_styled(&text.content, style, space),
        )
    }

    fn shape(
        &self,
        text: &super::LayoutText,
        known: crate::layout::style_data::Axes<Option<f32>>,
        wrap_to: Option<f32>,
    ) -> crate::layout::style_data::Axes<f32> {
        let width = if text.wrap {
            known.width.or(wrap_to).map_or_else(
                || space_constraint_to_hal_dim(self.available.w),
                crate::geometry::Dim::Fixed,
            )
        } else {
            crate::geometry::Dim::MaxContent
        };
        let measured = self.measure_at(text, width);
        let natural_w = if text.ellipsis {
            known
                .width
                .or(wrap_to)
                .map_or(measured.width, |box_w| measured.width.min(box_w))
        } else {
            measured.width
        };
        crate::layout::style_data::Axes {
            width: known.width.unwrap_or(natural_w),
            height: known.height.unwrap_or(measured.height),
        }
    }
}

const fn space_constraint_to_hal_dim(space: SpaceConstraint) -> crate::geometry::Dim {
    match space {
        SpaceConstraint::Definite(value) => crate::geometry::Dim::Fixed(value),
        SpaceConstraint::MinContent => crate::geometry::Dim::MinContent,
        SpaceConstraint::MaxContent => crate::geometry::Dim::MaxContent,
    }
}
