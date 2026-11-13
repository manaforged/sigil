use std::collections::HashMap;
use std::rc::Rc;

use crate::geometry::Rect;
#[cfg(host)]
use crate::kernel::FrameDelta;
use crate::kernel::{Cx, Node, NodeId, Surface, TopLayer};
use crate::layout::LayoutResult;
use crate::scene::canvas::CanvasPaint;
use crate::style::text_identity::DiscreteTextStyle;
use crate::style::{Color, RenderStyle, StyleBinding, TextStyle, Theme, ToneContext};

use crate::scene::builder::SceneBuilder;
use crate::scene::command::{BlendMode, CommandScene, LayerEffect, LayerScope};
use crate::scene::emit_cache;
use crate::scene::shadow::Shadow;
use crate::scene::style_adapter::{corner_radii_for, render_style_for};

mod background;
mod borders;
mod dashes;
mod paint;
mod runs;
mod selection;
#[cfg(feature = "scene")]
mod text_fill;
use paint::{NodePaint, push_focus_ring, push_node};
mod effects;
mod filter;
mod mask;
#[cfg(host)]
mod memo;
mod subtree;
mod text_clip;
mod top_layer;
#[cfg(host)]
pub(crate) use memo::{CleanScene, EmitMemo};
#[cfg(terminal_host)]
kittest_pub! {
    #[doc(hidden)]
    pub use subtree::EmitDiff;
}
#[cfg(host)]
use subtree::Prior;
use subtree::SpanTrack;
use text_clip::TextFill;

const VIEW_OVERSCAN: f32 = 64.0;

kittest_pub! {
    /// Emits the draw commands under `root`, skipping nodes outside `clip`.
    #[doc(hidden)]
    #[must_use]
    pub fn emit_scene(cx: &Cx, root: NodeId, clip: Option<Rect>) -> CommandScene {
        let selection = crate::events::selection::ranges(cx);
        let mut track = SpanTrack::off();
        run(cx, root, clip, &selection, &mut track)
    }
}

#[cfg(feature = "terminal")]
pub(crate) fn emit_unchanged(
    cx: &Cx,
    (root, clip): (NodeId, Option<Rect>),
    (delta, prev): (&FrameDelta, &CommandScene),
    memo: &EmitMemo,
) -> bool {
    let selection = crate::events::selection::ranges(cx);
    memo.clean(cx, (root, clip), (delta, prev.commands.len()), &selection)
}

#[cfg(host)]
fn emit_with_memo(
    cx: &Cx,
    (root, clip): (NodeId, Option<Rect>),
    (delta, prev): (&FrameDelta, &CommandScene),
    memo: &mut EmitMemo,
    want_diff: bool,
) -> (CommandScene, SpanTrack<'static>) {
    let selection = crate::events::selection::ranges(cx);
    let usable = memo.usable(cx, (root, clip), &prev.commands);
    let (can_copy, dirty) = memo.dirty(cx, (root, usable), delta, &selection);
    let spans = memo.spans.take();
    let mut next = memo.scratch.take().unwrap_or_default();
    next.clear();
    let prior = can_copy
        .then_some(())
        .and(spans.as_ref())
        .map(|spans| Prior {
            commands: &prev.commands,
            spans,
            dirty: &dirty,
        });
    let mut track = SpanTrack::recording(prior, next, want_diff);
    let scene = run(cx, root, clip, &selection, &mut track);
    let SpanTrack {
        next: built,
        #[cfg(terminal_host)]
        copies,
        ..
    } = track;
    memo.scratch = spans;
    memo.commit(cx, root, (built, scene.commands.len(), clip), &selection);
    #[cfg(terminal_host)]
    let out_track = SpanTrack::with_copies(copies);
    #[cfg(not(terminal_host))]
    let out_track = SpanTrack::off();
    (scene, out_track)
}

#[cfg(terminal_host)]
pub(crate) fn emit_scene_since(
    cx: &Cx,
    root: NodeId,
    clip: Option<Rect>,
    (delta, prev): (&FrameDelta, &CommandScene),
    memo: &mut EmitMemo,
) -> (CommandScene, EmitDiff) {
    let selection = crate::events::selection::ranges(cx);
    if memo.clean(cx, (root, clip), (delta, prev.commands.len()), &selection) {
        return (prev.clone(), EmitDiff::default());
    }
    let (scene, track) = emit_with_memo(cx, (root, clip), (delta, prev), memo, true);
    let diff = EmitDiff::between(&prev.commands, &scene.commands, &track.copies);
    (scene, diff)
}

#[cfg(feature = "desktop")]
pub(crate) fn emit_scene_into(
    cx: &Cx,
    (root, clip): (NodeId, Option<Rect>),
    delta: &FrameDelta,
    body: &mut CommandScene,
    memo: &mut EmitMemo,
) -> bool {
    let selection = crate::events::selection::ranges(cx);
    if memo.clean(cx, (root, clip), (delta, body.commands.len()), &selection) {
        return false;
    }
    let (scene, _) = emit_with_memo(cx, (root, clip), (delta, body), memo, false);
    *body = scene;
    true
}

fn run<'t>(
    cx: &'t Cx,
    root: NodeId,
    clip: Option<Rect>,
    selection: &HashMap<NodeId, (usize, usize)>,
    track: &mut SpanTrack<'t>,
) -> CommandScene {
    let clip = clip.map(|rect| rect.expand(VIEW_OVERSCAN));
    let mut scene = SceneBuilder::new();
    let theme = crate::style::theme::fallback_theme();
    let text_style = Rc::new(TextStyle::default());
    let text_style_stamp = DiscreteTextStyle::identity(&text_style);
    let inherited = Inherited {
        theme,
        theme_stamp: emit_cache::theme_key(theme, 0),
        text_color: None,
        text_style,
        text_style_stamp,
        text_fill: None,
        text_shadows: None,
    };
    let mut deferred: Vec<(NodeId, Inherited<'_>, Option<Rect>)> = Vec::new();
    let mut walk = EmitWalk {
        scene: &mut scene,
        cx,
        deferred: &mut deferred,
        selection,
        track,
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
    selection: &'a HashMap<NodeId, (usize, usize)>,
    track: &'a mut SpanTrack<'t>,
}

#[derive(Clone)]
struct Inherited<'t> {
    theme: &'t Theme,
    theme_stamp: u64,
    text_color: Option<Color>,
    text_style: Rc<TextStyle>,
    text_style_stamp: u64,
    text_fill: Option<Rc<TextFill>>,
    text_shadows: Option<Rc<[Shadow]>>,
}

impl<'t> EmitWalk<'_, 't> {
    fn push_subtree(&mut self, id: NodeId, inherited: &Inherited<'t>, clip: Option<Rect>) {
        if self.copy_subtree(id, inherited, clip) {
            return;
        }
        let open = self.open_span();
        self.walk_subtree(id, inherited, clip);
        self.close_span(id, inherited, clip, &open);
    }

    fn walk_subtree(&mut self, id: NodeId, inherited: &Inherited<'t>, clip: Option<Rect>) {
        let Some(node) = self.cx.get(id) else {
            return;
        };
        crate::profile::count(crate::profile::Counter::EmitNodes);
        self.track.walked();
        if skips_paint(node) {
            return;
        }
        let theme = node.extension::<Theme>().unwrap_or(inherited.theme);
        if node.extension::<StyleBinding>().is_none() && outside_clip(node, clip) {
            self.defer_top_layer(node, || {
                let style = render_style_for(node, theme, inherited.text_color);
                child_inherited(node, inherited, &style)
            });
            return;
        }
        let style = render_style_for(node, theme, inherited.text_color);
        let layered = (!style.transform.is_identity()
            || !style.space.is_flat()
            || style.opacity < 10_000
            || style.blend != BlendMode::Normal)
            && node.extension::<LayoutResult>().is_some();
        let cut = style.clip_path.is_some() && node.extension::<LayoutResult>().is_some();
        if style.opacity > 0 && layered {
            self.push_effect(id, inherited, clip, style);
        } else if style.opacity > 0 && (cut || !outside_clip(node, clip)) {
            self.push_masked(id, inherited, clip, style);
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
        if let Some(result) = node.extension::<LayoutResult>() {
            let rect = result.rect();
            let paint = NodePaint {
                style,
                theme,
                text_base: &inherited.text_style,
                tone: node.extension::<ToneContext>().map(|context| context.tone),
                leaf: !has_laid_child(self.cx, node),
                selected: self.selection.get(&id).copied(),
                text_fill: inherited.text_fill.as_deref().map(|fill| &fill.gradient),
                text_shadows: inherited.text_shadows.as_deref().unwrap_or_default(),
            };
            let inherit = paint_key(node, inherited, &paint);
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
            let key = slotmap::Key::data(&id.key()).as_ffi();
            crate::profile::attribute(crate::profile::Hotspot::EmitResolved, 1, key, || {
                cx.describe(id)
            });
        }
        push_focus_ring(self.scene, id, node, rect, paint);
        push_node(self.scene, id, node, rect, paint);
        if let Some(canvas) = node.extension::<CanvasPaint>() {
            canvas.record(self.scene, rect, paint.theme);
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
        let track = &mut *self.track;
        let body = |scene: &mut SceneBuilder| {
            EmitWalk {
                scene,
                cx,
                deferred,
                selection,
                track,
            }
            .push_children(node, inherited, Some(inner));
        };
        if clip_radii.0.iter().any(|radius| *radius > 0.0) {
            let scope = LayerScope {
                clip: rect,
                clip_radii,
                effect: LayerEffect::Identity,
                opacity: 1.0,
                blend: crate::scene::BlendMode::Normal,
            };
            self.scene.composite(scope, body);
        } else {
            self.scene.clip(rect, body);
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

fn paint_key(node: &Node, inherited: &Inherited<'_>, paint: &NodePaint<'_>) -> u64 {
    emit_cache::inherit_key(
        paint.theme,
        inherited.text_color,
        inherited.text_style.as_ref(),
    ) ^ emit_cache::content_key(node)
        ^ u64::from(paint.leaf)
        ^ node
            .extension::<CanvasPaint>()
            .map_or(0, CanvasPaint::version)
        ^ inherited.text_fill.as_ref().map_or(0, |fill| fill.key)
        ^ emit_cache::shadow_key(paint.text_shadows)
}

fn outside_clip(node: &Node, clip: Option<Rect>) -> bool {
    clip.zip(node.extension::<LayoutResult>())
        .is_some_and(|(clip, result)| !result.rect().intersects(clip))
}

fn has_laid_child(cx: &Cx, node: &Node) -> bool {
    node.children().iter().any(|child| {
        cx.get(*child).is_some_and(|child| {
            if child.extension::<Surface>().is_some() || is_top_layer(child) {
                return false;
            }
            child.extension::<LayoutResult>().is_some() || has_laid_child(cx, child)
        })
    })
}

fn is_display_none(node: &Node) -> bool {
    node.extension::<StyleBinding>()
        .is_some_and(StyleBinding::hides)
}

fn is_top_layer(node: &Node) -> bool {
    node.extension::<TopLayer>().is_some()
}

fn skips_paint(node: &Node) -> bool {
    is_display_none(node) || node.extension::<Surface>().is_some()
}

fn child_inherited<'t>(
    node: &'t Node,
    inherited: &Inherited<'t>,
    style: &RenderStyle,
) -> Inherited<'t> {
    let (text_style, text_style_stamp) = node
        .extension::<StyleBinding>()
        .filter(|binding| TextStyle::binding_layers_text(binding))
        .map_or_else(
            || (Rc::clone(&inherited.text_style), inherited.text_style_stamp),
            |binding| {
                let layered = emit_cache::layered_text(&inherited.text_style, binding);
                let stamp = DiscreteTextStyle::identity(&layered);
                (layered, stamp)
            },
        );
    let (theme, theme_stamp) =
        node.extension::<Theme>()
            .map_or((inherited.theme, inherited.theme_stamp), |theme| {
                (
                    theme,
                    emit_cache::theme_key(theme, u64::from(node.version())),
                )
            });
    let text_fill =
        TextFill::of(node, style, theme).map_or_else(|| inherited.text_fill.clone(), Some);
    let text_shadows = if style.filter.inherits_text_shadows() {
        inherited.text_shadows.clone()
    } else {
        Some(Rc::from(style.filter.text_shadows().as_slice()))
    };
    Inherited {
        theme,
        theme_stamp,
        text_color: style.text_color.or(inherited.text_color),
        text_style,
        text_style_stamp,
        text_fill,
        text_shadows,
    }
}
