use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{Cx, Node, NodeId, Surface, TopLayer};
use crate::layout::LayoutResult;
use crate::style::{RenderStyle, Rgba8, StyleBinding, TextStyle, Theme, ToneContext};

use crate::scene::builder::SceneBuilder;
use crate::scene::command::{CommandScene, LayerEffect, LayerScope};
use crate::scene::emit_cache;
use crate::scene::style_adapter::{corner_radii_for, render_style_for};

mod borders;
mod paint;
mod runs;
mod selection;
use paint::{NodePaint, push_focus_ring, push_node};
mod effects;
use effects::push_effect;
mod top_layer;
use top_layer::defer_top_layer;

const VIEW_OVERSCAN: f32 = 64.0;

#[must_use]
pub fn emit_scene(cx: &Cx, root: NodeId) -> CommandScene {
    emit_scene_clipped(cx, root, None)
}

#[must_use]
pub fn emit_scene_clipped(cx: &Cx, root: NodeId, clip: Option<Rect>) -> CommandScene {
    let clip = clip.map(|rect| rect.expand(VIEW_OVERSCAN));
    let mut scene = SceneBuilder::new();
    let selection = crate::events::selection::ranges(cx);
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
        selection: &selection,
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
    selection: &'a std::collections::HashMap<NodeId, (usize, usize)>,
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
    if skips_paint(node) {
        return;
    }
    let theme = node.get_extension::<Theme>().unwrap_or(inherited.theme);
    let style = render_style_for(id, node, theme, inherited.text_color);
    let layered = (!style.transform.is_identity() || style.opacity < 10_000)
        && node.get_extension::<LayoutResult>().is_some();
    if style.opacity > 0 && layered {
        push_effect(walk, id, inherited, clip, style);
    } else if style.opacity > 0 && !outside_clip(node, clip) {
        push_contents(walk, id, inherited, clip, &style);
    } else {
        defer_top_layer(walk, node, || child_inherited(node, inherited, &style));
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
    let child_context = child_inherited(node, inherited, style);
    let theme = child_context.theme;
    if let Some(result) = node.get_extension::<LayoutResult>() {
        let rect = result.rect();
        let paint = NodePaint {
            style,
            theme,
            text_base: &inherited.text_style,
            tone: node
                .get_extension::<ToneContext>()
                .map(|context| context.tone),
            leaf: !has_laid_child(walk.cx, node),
            selected: walk.selection.get(&id).copied(),
        };
        let inherit =
            emit_cache::inherit_key(theme, inherited.text_color, inherited.text_style.as_ref())
                ^ emit_cache::content_key(node)
                ^ u64::from(paint.leaf);
        let selected = paint.selected.is_some();
        let key = (walk.cx.instance_id(), id);
        if selected || !emit_cache::replay(walk.scene, key, node, rect, inherit) {
            let start = walk.scene.command_len();
            push_focus_ring(walk.scene, id, node, rect, &paint);
            push_node(walk.scene, id, node, rect, &paint);
            if !selected {
                emit_cache::store(key, node, rect, inherit, walk.scene.copy_from(start));
            }
        }
        if result.clips() {
            emit_clipped_children(walk, node, &child_context, clip, rect, style);
            return;
        }
    }
    push_children(walk, node, &child_context, clip);
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
        defer_top_layer(walk, node, || inherited.clone());
        return;
    };
    let inner = inner.expand(1.0);
    let clip_radii = corner_radii_for(style.border_radius);
    let cx = walk.cx;
    let deferred = &mut *walk.deferred;
    let selection = walk.selection;
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
                    selection,
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
                selection,
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
        if walk.cx.get(child).is_some_and(is_top_layer) {
            walk.deferred.push((child, inherited.clone(), None));
        } else {
            push_subtree(walk, child, inherited, clip);
        }
    }
}

fn has_laid_child(cx: &Cx, node: &Node) -> bool {
    node.children().iter().any(|child| {
        cx.get(*child).is_some_and(|child| {
            if child.get_extension::<Surface>().is_some() || is_top_layer(child) {
                return false;
            }
            child.get_extension::<LayoutResult>().is_some() || has_laid_child(cx, child)
        })
    })
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

fn is_top_layer(node: &Node) -> bool {
    node.get_extension::<TopLayer>().is_some()
}

fn skips_paint(node: &Node) -> bool {
    is_display_none(node) || node.get_extension::<Surface>().is_some()
}

fn child_inherited<'t>(
    node: &'t Node,
    inherited: &Inherited<'t>,
    style: &RenderStyle,
) -> Inherited<'t> {
    let layered_text = node
        .get_extension::<StyleBinding>()
        .and_then(|binding| TextStyle::layered_over_binding(&inherited.text_style, binding));
    Inherited {
        theme: node.get_extension::<Theme>().unwrap_or(inherited.theme),
        text_color: style.text_color.or(inherited.text_color),
        text_style: layered_text.map_or_else(|| Rc::clone(&inherited.text_style), Rc::new),
    }
}
