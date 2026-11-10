use crate::geometry::Rect;
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutResult;
use crate::scene::command::effects::{filter_reach, transform_rect};
use crate::scene::command::{LayerEffect, LayerScope};
use crate::scene::corner_radii::CornerRadii;
use crate::style::atom::SpaceStyle;
use crate::style::projection::projection;
use crate::style::{FocusRing, Metrics, RenderStyle, StyleAtom, StyleBinding, Transform2D};

use super::{EmitWalk, Inherited, child_inherited, is_display_none, is_top_layer};

impl<'t> EmitWalk<'_, 't> {
    pub(super) fn push_effect(
        &mut self,
        id: NodeId,
        inherited: &Inherited<'t>,
        clip: Option<Rect>,
        mut style: RenderStyle,
    ) {
        let Some(node) = self.cx.get(id) else {
            return;
        };
        let Some(result) = node.get_extension::<LayoutResult>() else {
            return;
        };
        let rect = result.rect();
        let reach = filter_reach(&style.filter.filters());
        let content = content_bounds(self.cx, id)
            .unwrap_or(rect)
            .expand(1.0 + reach);
        let (effect, screen) = layer_effect(self.cx, id, &style, rect, content);
        let flat = style.transform.is_identity() && style.space.is_flat();
        let Some(bounds) = visible_bounds(content, clip, screen, flat) else {
            self.defer_top_layer(node, || child_inherited(node, inherited, &style));
            return;
        };
        let opacity = f32::from(style.opacity) / 10_000.0;
        style.opacity = 10_000;
        let blend = std::mem::take(&mut style.blend);
        style.transform = Transform2D::IDENTITY;
        style.space = SpaceStyle::FLAT;
        let cx = self.cx;
        let deferred = &mut *self.deferred;
        let selection = self.selection;
        let track = &mut *self.track;
        self.scene.with_composite_layer(
            LayerScope {
                clip: bounds,
                clip_radii: CornerRadii::ZERO,
                effect,
                opacity,
                blend,
            },
            |scene| {
                EmitWalk {
                    scene,
                    cx,
                    deferred,
                    selection,
                    track,
                }
                .push_masked(id, inherited, None, style);
            },
        );
    }
}

fn layer_effect(
    cx: &Cx,
    id: NodeId,
    style: &RenderStyle,
    rect: Rect,
    content: Rect,
) -> (LayerEffect, Option<Rect>) {
    if let Some((transform, hide_backface)) =
        projection(cx, id, (style.transform, &style.space), rect)
    {
        (
            LayerEffect::Projected {
                transform,
                hide_backface,
            },
            transform.bounds(content),
        )
    } else {
        let transform = style.transform.matrix(rect);
        (
            LayerEffect::Transform(transform),
            Some(transform_rect(transform, content)),
        )
    }
}

fn visible_bounds(
    bounds: Rect,
    clip: Option<Rect>,
    screen: Option<Rect>,
    identity: bool,
) -> Option<Rect> {
    let Some(clip) = clip else {
        return Some(bounds);
    };
    let bounds = if identity {
        bounds.intersection(clip)?
    } else {
        bounds
    };
    screen?.intersects(clip).then_some(bounds)
}

pub(super) fn content_bounds(cx: &Cx, id: NodeId) -> Option<Rect> {
    let node = cx.get(id)?;
    if is_display_none(node) {
        return None;
    }
    let result = node.get_extension::<LayoutResult>()?;
    let mut bounds = result.rect();
    if let Some(elevation) = node.get_extension::<StyleBinding>().and_then(|binding| {
        binding
            .cascade()
            .filter_map(|atom| match atom {
                StyleAtom::Shadow(elevation) => Some(elevation),
                _ => None,
            })
            .last()
    }) {
        bounds = bounds.union(crate::scene::shadow::expanded_shadow_rect(
            result.rect(),
            super::paint::elevation_shadow(elevation),
        ));
    }
    if let Some(ring) = node
        .get_extension::<FocusRing>()
        .filter(|ring| ring.visible())
    {
        bounds = bounds.union(result.rect().expand(
            ring.offset_px(Metrics::current()).max(0.0)
                + ring.stroke_width(Metrics::current()).max(0.0),
        ));
    }
    if !result.clips() {
        for child in node.children() {
            if let Some(child_bounds) = transformed_bounds(cx, *child) {
                bounds = bounds.union(child_bounds);
            }
        }
    }
    Some(bounds)
}

fn transformed_bounds(cx: &Cx, id: NodeId) -> Option<Rect> {
    let node = cx.get(id)?;
    if is_top_layer(node) {
        return None;
    }
    let bounds = content_bounds(cx, id)?;
    let rect = node.get_extension::<LayoutResult>()?.rect();
    let Some(binding) = node.get_extension::<StyleBinding>() else {
        return Some(bounds);
    };
    let flat = Transform2D::for_binding(binding);
    let space = SpaceStyle::for_binding(binding);
    match projection(cx, id, (flat, &space), rect) {
        Some((transform, _)) => transform.bounds(bounds),
        None => Some(transform_rect(flat.matrix(rect), bounds)),
    }
}
