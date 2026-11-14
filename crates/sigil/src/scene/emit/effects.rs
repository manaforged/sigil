use crate::geometry::Rect;
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutResult;
use crate::scene::command::effects::{filter_reach, transform_rect};
use crate::scene::command::{LayerEffect, LayerScope};
use crate::scene::corner_radii::CornerRadii;
use crate::style::atom::{OPACITY_UNITS, SpaceStyle};
use crate::style::projection::{node_plane, planes, preserves, projection};
use crate::style::{FocusRing, Metrics, RenderStyle, StyleAtom, StyleBinding, Transform2D};

use super::mask::LayerOrClip;
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
        let Some(result) = node.extension::<LayoutResult>() else {
            return;
        };
        let rect = result.rect();
        let preserve = style.preserves_3d(result.clips());
        let reach = filter_reach(&style.filter.filters());
        let content = content_bounds(self.cx, id)
            .unwrap_or(rect)
            .expand(1.0 + reach);
        let (effect, screen) = layer_effect(self.cx, id, &style, rect, content);
        let flat =
            style.transform.is_identity() && !matches!(effect, LayerEffect::Projected { .. });
        let nested = preserve.then(|| child_inherited(node, inherited, &style));
        if let Some(bounds) = visible_bounds(content, clip, screen, flat) {
            let scope = LayerScope {
                clip: bounds,
                clip_radii: CornerRadii::ZERO,
                effect,
                opacity: f32::from(style.opacity) / OPACITY_UNITS,
                blend: std::mem::take(&mut style.blend),
            };
            style.opacity = 10_000;
            style.transform = Transform2D::IDENTITY;
            style.space = SpaceStyle {
                preserve,
                ..SpaceStyle::FLAT
            };
            self.nested(LayerOrClip::Layer(scope), |walk| {
                walk.push_masked(id, inherited, None, style);
            });
        } else if nested.is_none() {
            self.defer_top_layer(node, || child_inherited(node, inherited, &style));
        }
        if let Some(nested) = nested {
            self.push_nodes(planes(self.cx, id), &nested, clip);
        }
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
    let result = node.extension::<LayoutResult>()?;
    let mut bounds = crate::animation::runtime::layout_motion::paint_bounds(node, result.rect());
    if let Some(shadow) = node.extension::<StyleBinding>().and_then(|binding| {
        binding
            .cascade()
            .filter_map(|atom| match atom {
                StyleAtom::Shadow(shadow) => Some(shadow),
                _ => None,
            })
            .last()
    }) {
        bounds = bounds.union(crate::scene::shadow::expanded_shadow_rect(
            result.rect(),
            shadow.into(),
        ));
    }
    if let Some(ring) = node.extension::<FocusRing>().filter(|ring| ring.visible()) {
        bounds = bounds.union(result.rect().expand(
            ring.offset_px(Metrics::current()).max(0.0)
                + ring.stroke_width(Metrics::current()).max(0.0),
        ));
    }
    if !result.clips() && !preserves(cx, id) {
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
    let mut screen = match node_plane(cx, id) {
        Some((transform, _)) => transform.bounds(bounds),
        None => Some(bounds),
    };
    if preserves(cx, id) {
        for child in node.children() {
            if let Some(plane) = transformed_bounds(cx, *child) {
                screen = Some(screen.map_or(plane, |screen| screen.union(plane)));
            }
        }
    }
    screen
}
