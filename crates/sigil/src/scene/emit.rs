use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::{Cx, Node, NodeId, Surface, TopLayer};
use crate::layout::LayoutResult;
use crate::style::{RenderStyle, Rgba8, StyleBinding, TextStyle, Theme, ToneContext};
use crate::widgets::CanvasPaint;

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
mod top_layer;

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
    walk.push_subtree(root, &inherited, clip);
    let mut next = 0;
    while let Some((id, inh, child_clip)) = walk.deferred.get(next).cloned() {
        next += 1;
        walk.push_subtree(id, &inh, child_clip);
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

impl<'t> EmitWalk<'_, 't> {
    fn push_subtree(&mut self, id: NodeId, inherited: &Inherited<'t>, clip: Option<Rect>) {
        let Some(node) = self.cx.get(id) else {
            return;
        };
        crate::profile::count(crate::profile::Counter::EmitNodes);
        if skips_paint(node) {
            return;
        }
        let theme = node.get_extension::<Theme>().unwrap_or(inherited.theme);
        let style = render_style_for(id, node, theme, inherited.text_color);
        let layered = (!style.transform.is_identity() || style.opacity < 10_000)
            && node.get_extension::<LayoutResult>().is_some();
        if style.opacity > 0 && layered {
            self.push_effect(id, inherited, clip, style);
        } else if style.opacity > 0 && !outside_clip(node, clip) {
            self.push_contents(id, inherited, clip, &style);
        } else {
            self.defer_top_layer(node, || child_inherited(node, inherited, &style));
        }
    }

    fn push_contents(
        &mut self,
        id: NodeId,
        inherited: &Inherited<'t>,
        clip: Option<Rect>,
        style: &RenderStyle,
    ) {
        let Some(node) = self.cx.get(id) else {
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
                leaf: !has_laid_child(self.cx, node),
                selected: self.selection.get(&id).copied(),
            };
            let inherit =
                emit_cache::inherit_key(theme, inherited.text_color, inherited.text_style.as_ref())
                    ^ emit_cache::content_key(node)
                    ^ u64::from(paint.leaf)
                    ^ node
                        .get_extension::<CanvasPaint>()
                        .map_or(0, CanvasPaint::version);
            let selected = paint.selected.is_some();
            let key = (self.cx.instance_id(), id);
            if selected || !emit_cache::replay(self.scene, key, node, rect, inherit) {
                let start = self.scene.command_len();
                self.record_node(id, node, rect, &paint);
                if !selected {
                    emit_cache::store(key, node, rect, inherit, self.scene.copy_from(start));
                }
            }
            if result.clips() {
                self.emit_clipped_children(node, &child_context, clip, rect, style);
                return;
            }
        }
        self.push_children(node, &child_context, clip);
    }

    fn record_node(&mut self, id: NodeId, node: &Node, rect: Rect, paint: &NodePaint<'_>) {
        if crate::profile::enabled() {
            let cx = self.cx;
            crate::profile::attribute(crate::profile::Hotspot::EmitResolved, 1, || cx.describe(id));
        }
        push_focus_ring(self.scene, id, node, rect, paint);
        push_node(self.scene, id, node, rect, paint);
        if let Some(canvas) = node.get_extension::<CanvasPaint>() {
            canvas.record(self.scene, rect);
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "clip self needs the node, inherit, and both clip rects"
    )]
    fn emit_clipped_children(
        &mut self,
        node: &Node,
        inherited: &Inherited<'t>,
        clip: Option<Rect>,
        rect: Rect,
        style: &RenderStyle,
    ) {
        let Some(inner) = clip.map_or(Some(rect), |clip| clip.intersection(rect)) else {
            self.defer_top_layer(node, || inherited.clone());
            return;
        };
        let inner = inner.expand(1.0);
        let clip_radii = corner_radii_for(style.border_radius);
        let (cx, deferred, selection) = (self.cx, &mut *self.deferred, self.selection);
        let body = |scene: &mut SceneBuilder| {
            EmitWalk {
                scene,
                cx,
                deferred,
                selection,
            }
            .push_children(node, inherited, Some(inner));
        };
        if clip_radii.0.iter().any(|radius| *radius > 0.0) {
            let scope = LayerScope {
                clip: rect,
                clip_radii,
                effect: LayerEffect::Identity,
                opacity: 1.0,
            };
            self.scene.with_composite_layer(scope, body);
        } else {
            self.scene.with_clip(rect, body);
        }
    }

    fn push_children(&mut self, node: &Node, inherited: &Inherited<'t>, clip: Option<Rect>) {
        for child in node.children().iter().copied() {
            if self.cx.get(child).is_some_and(is_top_layer) {
                self.deferred.push((child, inherited.clone(), None));
            } else {
                self.push_subtree(child, inherited, clip);
            }
        }
    }
}

fn outside_clip(node: &Node, clip: Option<Rect>) -> bool {
    clip.zip(node.get_extension::<LayoutResult>())
        .is_some_and(|(clip, result)| !result.rect().intersects(clip))
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
    let text_style = node
        .get_extension::<StyleBinding>()
        .filter(|binding| TextStyle::binding_layers_text(binding))
        .map_or_else(
            || Rc::clone(&inherited.text_style),
            |binding| emit_cache::layered_text(&inherited.text_style, binding),
        );
    Inherited {
        theme: node.get_extension::<Theme>().unwrap_or(inherited.theme),
        text_color: style.text_color.or(inherited.text_color),
        text_style,
    }
}
