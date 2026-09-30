use crate::geometry::Rect;
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutResult;
use crate::scene::command::effects::{Transform, transform_rect};
use crate::scene::command::{LayerEffect, LayerScope};
use crate::scene::corner_radii::CornerRadii;
use crate::style::{FocusRing, Metrics, RenderStyle, StyleAtom, StyleBinding, Transform2D};

use super::{
    EmitWalk, Inherited, child_inherited, defer_top_layer, is_display_none, is_top_layer,
    push_contents,
};

pub(super) fn push_effect<'t>(
    walk: &mut EmitWalk<'_, 't>,
    id: NodeId,
    inherited: &Inherited<'t>,
    clip: Option<Rect>,
    mut style: RenderStyle,
) {
    let Some(node) = walk.cx.get(id) else {
        return;
    };
    let Some(result) = node.get_extension::<LayoutResult>() else {
        return;
    };
    let rect = result.rect();
    let transform = style.transform.matrix(rect);
    let content = content_bounds(walk.cx, id).unwrap_or(rect).expand(1.0);
    let Some(bounds) = visible_bounds(content, clip, transform, style.transform.is_identity())
    else {
        defer_top_layer(walk, node, || child_inherited(node, inherited, &style));
        return;
    };
    let opacity = f32::from(style.opacity) / 10_000.0;
    style.opacity = 10_000;
    style.transform = Transform2D::IDENTITY;
    let cx = walk.cx;
    let deferred = &mut *walk.deferred;
    let selection = walk.selection;
    walk.scene.with_composite_layer(
        LayerScope {
            clip: bounds,
            clip_radii: CornerRadii::ZERO,
            effect: LayerEffect::Transform(transform),
            opacity,
        },
        |scene| {
            push_contents(
                &mut EmitWalk {
                    scene,
                    cx,
                    deferred,
                    selection,
                },
                id,
                inherited,
                None,
                &style,
            );
        },
    );
}

fn visible_bounds(
    bounds: Rect,
    clip: Option<Rect>,
    transform: Transform,
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
    transform_rect(transform, bounds)
        .intersects(clip)
        .then_some(bounds)
}

fn content_bounds(cx: &Cx, id: NodeId) -> Option<Rect> {
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
    Some(
        node.get_extension::<StyleBinding>()
            .map_or(bounds, |binding| {
                transform_rect(Transform2D::for_binding(binding).matrix(rect), bounds)
            }),
    )
}
