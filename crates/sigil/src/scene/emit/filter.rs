use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::layout::LayoutResult;
use crate::scene::command::effects::{filter_reach, transform_rect};
use crate::scene::command::{BlendMode, LayerEffect, LayerScope};
use crate::scene::corner_radii::CornerRadii;
use crate::scene::style_adapter::corner_radii_for;
use crate::style::RenderStyle;
use crate::style::atom::OPACITY_UNITS;

use super::effects::content_bounds;
use super::mask::LayerOrClip;
use super::{EmitWalk, Inherited};

impl<'t> EmitWalk<'_, 't> {
    pub(super) fn push_filtered(
        &mut self,
        id: NodeId,
        inherited: &Inherited<'t>,
        clip: Option<Rect>,
        style: &RenderStyle,
    ) {
        let filters = style.filter.filters();
        let rect = self
            .cx
            .get(id)
            .and_then(|node| node.extension::<LayoutResult>())
            .map(LayoutResult::rect);
        let Some(rect) = rect.filter(|_| !filters.is_empty()) else {
            self.push_contents(id, inherited, clip, style);
            return;
        };
        let content = content_bounds(self.cx, id).unwrap_or(rect);
        let scope = LayerScope {
            clip: content.expand(1.0 + filter_reach(&filters)),
            clip_radii: CornerRadii::ZERO,
            effect: LayerEffect::Filter(filters.into_vec().into()),
            opacity: 1.0,
            blend: BlendMode::Normal,
        };
        self.nested(LayerOrClip::Layer(scope), |walk| {
            walk.push_contents(id, inherited, clip, style);
        });
    }

    pub(super) fn push_backdrop(&mut self, id: NodeId, style: &RenderStyle) {
        let filters = style.filter.backdrop();
        if filters.is_empty() {
            return;
        }
        let Some(rect) = self
            .cx
            .get(id)
            .and_then(|node| node.extension::<LayoutResult>())
            .map(LayoutResult::rect)
        else {
            return;
        };
        let clip = if style.space.is_flat() {
            transform_rect(style.transform.matrix(rect), rect)
        } else {
            rect
        };
        let scope = LayerScope {
            clip,
            clip_radii: corner_radii_for(style.border_radius),
            effect: LayerEffect::Backdrop(filters.into_vec().into()),
            opacity: f32::from(style.opacity) / OPACITY_UNITS,
            blend: BlendMode::Normal,
        };
        self.scene.composite(scope, |_| {});
    }
}
