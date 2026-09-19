use std::rc::Rc;

use crate::hal::Rect;
use crate::kernel::{Cx, Node, NodeId, Surface};
use crate::layout::LayoutResult;
use crate::layout::ViewportOverlay;
use crate::style::{RenderStyle, Rgba8, StyleBinding, TextStyle, Theme};

use crate::renderer::emit_cache;
use crate::renderer::scene_builder::SceneBuilder;
use crate::renderer::scene_command::{CommandScene, LayerEffect, LayerScope};
use crate::renderer::style_adapter::{corner_radii_for, render_style_for};

mod paint;
use paint::{NodePaint, push_focus_ring, push_node};
mod effects;
use effects::push_effect;

const VIEW_OVERSCAN: f32 = 64.0;

#[must_use]
pub fn emit_scene(cx: &Cx, root: NodeId) -> CommandScene {
    emit_scene_clipped(cx, root, None)
}

#[must_use]
pub fn emit_scene_clipped(cx: &Cx, root: NodeId, clip: Option<Rect>) -> CommandScene {
    let clip = clip.map(|rect| rect.expand(VIEW_OVERSCAN));
    let mut scene = SceneBuilder::new();
    let inherited = Inherited {
        theme: default_inherited_theme(),
        text_color: None,
        text_style: Rc::new(TextStyle::default()),
    };
    let mut deferred: Vec<(NodeId, Inherited<'_>, Option<Rect>)> = Vec::new();
    let mut walk = EmitWalk {
        scene: &mut scene,
        cx,
        deferred: &mut deferred,
    };
    push_subtree(&mut walk, root, &inherited, clip);
    let mut next = 0;
    while let Some((id, inh, child_clip)) = walk.deferred.get(next).cloned() {
        next += 1;
        push_subtree(&mut walk, id, &inh, child_clip);
    }
    scene.finish()
}

struct EmitWalk<'a, 't> {
    scene: &'a mut SceneBuilder,
    cx: &'t Cx,
    deferred: &'a mut Vec<(NodeId, Inherited<'t>, Option<Rect>)>,
}

#[derive(Clone)]
struct Inherited<'t> {
    theme: &'t Theme,
    text_color: Option<Rgba8>,
    text_style: Rc<TextStyle>,
}

fn push_subtree<'t>(
    walk: &mut EmitWalk<'_, 't>,
    id: NodeId,
    inherited: &Inherited<'t>,
    clip: Option<Rect>,
) {
    let Some(node) = walk.cx.get(id) else {
        return;
    };
    if is_display_none(node) || node.get_extension::<Surface>().is_some() {
        return;
    }
    let theme = node.get_extension::<Theme>().unwrap_or(inherited.theme);
    let style = render_style_for(id, node, theme, inherited.text_color);
    if style.opacity == 0 {
        return;
    }
    if (!style.transform.is_identity() || style.opacity < 10_000)
        && node.get_extension::<LayoutResult>().is_some()
    {
        push_effect(walk, id, inherited, clip, style);
    } else if !outside_clip(node, clip) {
        push_contents(walk, id, inherited, clip, &style);
    }
}

fn push_contents<'t>(
    walk: &mut EmitWalk<'_, 't>,
    id: NodeId,
    inherited: &Inherited<'t>,
    clip: Option<Rect>,
    style: &RenderStyle,
) {
    let Some(node) = walk.cx.get(id) else {
        return;
    };
    let theme = node.get_extension::<Theme>().unwrap_or(inherited.theme);
    let layered_text = node
        .get_extension::<StyleBinding>()
        .and_then(|binding| TextStyle::layered_over_binding(&inherited.text_style, binding));
    let child_inherited = Inherited {
        theme,
        text_color: style.text_color.or(inherited.text_color),
        text_style: layered_text.map_or_else(|| Rc::clone(&inherited.text_style), Rc::new),
    };
    if let Some(result) = node.get_extension::<LayoutResult>() {
        let rect = result.rect();
        let paint = NodePaint {
            style,
            theme,
            text_base: &inherited.text_style,
        };
        let inherit =
            emit_cache::inherit_key(theme, inherited.text_color, inherited.text_style.as_ref())
                ^ emit_cache::content_key(node);
        let key = (walk.cx.instance_id(), id);
        if !emit_cache::replay(walk.scene, key, node, rect, inherit) {
            let start = walk.scene.command_len();
            push_focus_ring(walk.scene, id, node, rect, &paint);
            push_node(walk.scene, id, node, rect, &paint);
            emit_cache::store(key, node, rect, inherit, walk.scene.copy_from(start));
        }
        if result.clips() {
            emit_clipped_children(walk, node, &child_inherited, clip, rect, style);
            return;
        }
    }
    push_children(walk, node, &child_inherited, clip);
}

#[allow(
    clippy::too_many_arguments,
    reason = "clip walk needs the node, inherit, and both clip rects"
)]
fn emit_clipped_children<'t>(
    walk: &mut EmitWalk<'_, 't>,
    node: &Node,
    inherited: &Inherited<'t>,
    clip: Option<Rect>,
    rect: Rect,
    style: &RenderStyle,
) {
    let Some(inner) = tighten_clip(clip, rect) else {
        return;
    };
    let inner = inner.expand(1.0);
    let clip_radii = corner_radii_for(style.border_radius);
    let cx = walk.cx;
    let deferred = &mut *walk.deferred;
    if clip_radii.0.iter().any(|radius| *radius > 0.0) {
        walk.scene.with_composite_layer(
            LayerScope {
                clip: rect,
                clip_radii,
                effect: LayerEffect::Identity,
                opacity: 1.0,
            },
            |scene| {
                let mut nested = EmitWalk {
                    scene,
                    cx,
                    deferred,
                };
                push_children(&mut nested, node, inherited, Some(inner));
            },
        );
    } else {
        walk.scene.with_clip(rect, |scene| {
            let mut nested = EmitWalk {
                scene,
                cx,
                deferred,
            };
            push_children(&mut nested, node, inherited, Some(inner));
        });
    }
}

fn outside_clip(node: &Node, clip: Option<Rect>) -> bool {
    let Some(clip) = clip else {
        return false;
    };
    let Some(result) = node.get_extension::<LayoutResult>() else {
        return false;
    };
    !result.rect().intersects(clip)
}

fn tighten_clip(clip: Option<Rect>, rect: Rect) -> Option<Rect> {
    clip.map_or(Some(rect), |clip| clip.intersection(rect))
}

fn push_children<'t>(
    walk: &mut EmitWalk<'_, 't>,
    node: &Node,
    inherited: &Inherited<'t>,
    clip: Option<Rect>,
) {
    for child in node.children().iter().copied() {
        let is_top_layer = walk
            .cx
            .get(child)
            .is_some_and(|n| n.get_extension::<crate::kernel::TopLayer>().is_some());
        if is_top_layer {
            let clip = walk
                .cx
                .get(child)
                .filter(|child_node| child_node.get_extension::<ViewportOverlay>().is_some())
                .map_or(clip, |_| None);
            walk.deferred.push((child, inherited.clone(), clip));
        } else {
            push_subtree(walk, child, inherited, clip);
        }
    }
}

fn default_inherited_theme<'theme>() -> &'theme Theme {
    use std::sync::OnceLock;
    static DEFAULT: OnceLock<Theme> = OnceLock::new();
    DEFAULT.get_or_init(Theme::default)
}

fn is_display_none(node: &Node) -> bool {
    node.get_extension::<StyleBinding>()
        .is_some_and(StyleBinding::hides)
}
