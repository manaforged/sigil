use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::layout::LayoutResult;
use crate::scene::command::effects::filter_reach;
use crate::scene::command::{BlendMode, LayerEffect, LayerScope};
use crate::scene::corner_radii::CornerRadii;
use crate::style::RenderStyle;

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
}
