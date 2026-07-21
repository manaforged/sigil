//! Headless scene emission for renderer integration tests.

use std::rc::Rc;

use sigil_hal::{Point, Rect};
use sigil_kernel::{Cx, Node, NodeId, NodeType, TextContent};
use sigil_layout::LayoutResult;
use sigil_style::{DisplayKind, RenderStyle, Rgba8, StyleAtom, StyleBinding, TextStyle, Theme};
use sigil_text::{OwnedTextEngine, TextConfig};

use crate::corner_radii::CornerRadii;
use crate::scene_builder::SceneBuilder;
use crate::scene_command::{
    CommandScene, GlyphRun, SceneRectStyle, Stroke, StrokeAlign, StrokeStyle,
};
use crate::style_adapter::{
    ResolvedFocusRing, corner_radii_for, focus_ring_for, render_style_for, text_style_for,
};

/// A headless list of primitives that would be submitted to the renderer.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Scene {
    items: Vec<SceneItem>,
}

impl Scene {
    /// Borrow emitted scene items.
    #[must_use]
    pub fn items(&self) -> &[SceneItem] {
        &self.items
    }

    pub(crate) fn push(&mut self, item: SceneItem) {
        self.items.push(item);
    }
}

/// Headless renderer primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneItem {
    /// Filled rectangle.
    Rect(SceneRect),
    /// Text run.
    Text(SceneText),
}

/// Filled rectangle emitted from a container node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneRect {
    /// Source node.
    pub node: NodeId,
    /// Layout rectangle.
    pub rect: Rect,
    /// Resolved fill colour.
    pub color: Rgba8,
    /// Corner radii in top-left, top-right, bottom-right, bottom-left order.
    pub border_radius: CornerRadii,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
}

/// Text run emitted from a text node.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneText {
    /// Source node.
    pub node: NodeId,
    /// Layout rectangle.
    pub rect: Rect,
    /// Text content.
    pub text: String,
    /// Resolved text colour.
    pub color: Rgba8,
    /// Resolved typography.
    pub style: TextStyle,
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
}

/// Emit a command-buffer scene for a laid-out core subtree.
#[must_use]
pub fn emit_scene(cx: &Cx, root: NodeId) -> CommandScene {
    let mut scene = SceneBuilder::new();
    let inherited = Inherited {
        theme: default_inherited_theme(),
        text_color: None,
        text_style: Rc::new(TextStyle::default()),
    };
    // `TopLayer` subtrees are deferred here and emitted after the main
    // walk, so floating UI (dropdown popups) paints above every later
    // sibling. Index loop: emitting a deferred subtree may discover
    // nested top-layer descendants and grow the queue.
    let mut deferred: Vec<(NodeId, Inherited<'_>)> = Vec::new();
    push_subtree(&mut scene, cx, root, &inherited, &mut deferred);
    let mut next = 0;
    while let Some((id, inh)) = deferred.get(next).cloned() {
        next += 1;
        push_subtree(&mut scene, cx, id, &inh, &mut deferred);
    }
    scene.finish()
}

/// Style context cascading down the scene walk: the effective theme,
/// the nearest ancestor's resolved text color, and the effective
/// inherited typography (DR-056). The color and typography halves are
/// the CSS-style inheritance cascade — a container's `text-*` /
/// `font-*` / `leading-*` classes style every descendant text node
/// that doesn't set its own. Typography resolution mirrors the layout
/// engine's measurement seed so text paints at the size it measured.
#[derive(Clone)]
struct Inherited<'t> {
    theme: &'t Theme,
    text_color: Option<Rgba8>,
    // `Rc`, not a borrow: the top-layer deferral queue stores a
    // subtree's inherited context past the stack frame that layered
    // it, so the effective typography must own its data. Containers
    // without typography atoms just bump the refcount.
    text_style: Rc<TextStyle>,
}

fn push_subtree<'t>(
    scene: &mut SceneBuilder,
    cx: &'t Cx,
    id: NodeId,
    inherited: &Inherited<'t>,
    deferred: &mut Vec<(NodeId, Inherited<'t>)>,
) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if is_display_none(node) {
        return;
    }
    // Theme inheritance: if this node attached its own `Theme`, that
    // becomes the effective theme for itself AND its descendants;
    // otherwise we keep the inherited one. Without this every
    // descendant ignored an ancestor's `.theme(Theme::dark())` and
    // resolved colors against the default light theme.
    let theme = node.get_extension::<Theme>().unwrap_or(inherited.theme);
    // Resolve once, whether or not this node paints: even a layout-less
    // container contributes its `text-*` classes to the color cascade.
    let style = render_style_for(id, node, theme, inherited.text_color);
    // DR-056 typography inheritance: layer this node's typography atoms
    // over the inherited style. `None` (no typography atoms — the
    // common container case) passes the inherited value through by
    // reference without cloning.
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
        push_focus_ring(scene, id, node, rect, theme);
        push_node(
            scene,
            id,
            node,
            rect,
            &NodePaint {
                style: &style,
                theme,
                text_base: &inherited.text_style,
            },
        );
        // `overflow: hidden`/`scroll` → scissor descendants to this node's box, so a
        // too-tall child (a long file in the editor pane, scrolled-out content) is
        // cut at the edge instead of painting over its siblings. The command walker
        // skips any glyph run / fill whose rect falls outside the active clip.
        if result.clips() {
            scene.with_clip(rect, |scene| {
                push_children(scene, cx, node, &child_inherited, deferred);
            });
            return;
        }
    }
    push_children(scene, cx, node, &child_inherited, deferred);
}

fn push_children<'t>(
    scene: &mut SceneBuilder,
    cx: &'t Cx,
    node: &Node,
    inherited: &Inherited<'t>,
    deferred: &mut Vec<(NodeId, Inherited<'t>)>,
) {
    for child in node.children().iter().copied() {
        // Deferral happens at the parent's iteration, so emitting a
        // deferred subtree root directly (from `emit_scene`'s queue
        // drain) never re-defers it, while nested `TopLayer` descendants
        // still queue behind it.
        let is_top_layer = cx
            .get(child)
            .is_some_and(|n| n.get_extension::<sigil_kernel::TopLayer>().is_some());
        if is_top_layer {
            deferred.push((child, inherited.clone()));
        } else {
            push_subtree(scene, cx, child, inherited, deferred);
        }
    }
}

fn default_inherited_theme<'theme>() -> &'theme Theme {
    use std::sync::OnceLock;
    static DEFAULT: OnceLock<Theme> = OnceLock::new();
    DEFAULT.get_or_init(Theme::default)
}

/// Return  if any atom slice on the node sets .
fn is_display_none(node: &Node) -> bool {
    let Some(binding) = node.get_extension::<StyleBinding>() else {
        return false;
    };
    let target = StyleAtom::Display(DisplayKind::None);
    binding.static_slice().contains(&target)
        || binding.dynamic_slice().contains(&target)
        || binding.reactive_contains(&target)
}

fn push_focus_ring(
    scene: &mut SceneBuilder,
    id: NodeId,
    node: &Node,
    rect: Rect,
    inherited: &Theme,
) {
    let Some(ring) = focus_ring_for(node, inherited).filter(|ring| ring.width > 0.0) else {
        return;
    };
    for ring_rect in focus_ring_rects(rect, ring) {
        scene.draw_rect_for_node(
            id,
            ring_rect,
            SceneRectStyle::new(ring.color, CornerRadii::ZERO, ring.opacity),
        );
    }
}

fn focus_ring_rects(rect: Rect, ring: ResolvedFocusRing) -> [Rect; 4] {
    let width = ring.width;
    let offset = ring.offset;
    let outer_x = rect.origin.x - offset - width;
    let outer_y = rect.origin.y - offset - width;
    let inner_y = rect.origin.y - offset;
    let right_x = rect.origin.x + rect.width + offset;
    let bottom_y = rect.origin.y + rect.height + offset;
    let outer_width = (offset + width).mul_add(2.0, rect.width);
    let side_height = offset.mul_add(2.0, rect.height);
    [
        Rect::new(Point::new(outer_x, outer_y), outer_width, width),
        Rect::new(Point::new(outer_x, bottom_y), outer_width, width),
        Rect::new(Point::new(outer_x, inner_y), width, side_height),
        Rect::new(Point::new(right_x, inner_y), width, side_height),
    ]
}

/// The resolved paint inputs for one node: its render style, the
/// effective theme (still needed for glyph-run text styling), and the
/// typography inherited from ancestors (DR-056) that the node's own
/// atoms layer over.
struct NodePaint<'a> {
    style: &'a RenderStyle,
    theme: &'a Theme,
    text_base: &'a TextStyle,
}

fn push_node(scene: &mut SceneBuilder, id: NodeId, node: &Node, rect: Rect, paint: &NodePaint<'_>) {
    let style = paint.style;
    match &node.kind {
        NodeType::Container => {
            let stroke = border_stroke(style, paint.theme);
            if style.background_color.is_some() || stroke.is_some() {
                // A border with no background paints a transparent
                // interior; the SDF fill contributes zero coverage and
                // only the stroke shows.
                let fill = style.background_color.unwrap_or(Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                });
                let mut rect_style =
                    SceneRectStyle::new(fill, corner_radii_for(style.border_radius), style.opacity);
                if let Some(stroke) = stroke {
                    rect_style = rect_style.with_stroke(stroke);
                }
                scene.draw_rect_for_node(id, rect, rect_style);
            }
        }
        NodeType::Text { content, .. } => {
            if let Some(color) = style.text_color {
                scene.draw_glyph_run_for_node(
                    id,
                    shape_glyph_run(
                        rect,
                        text_content(content),
                        color,
                        text_style_for(node, paint.theme, paint.text_base),
                        style.opacity,
                    ),
                );
            }
        }
        NodeType::Fragment => {}
    }
}

#[doc(hidden)]
/// Shape a text run for command-scene emission.
#[must_use]
pub fn shape_glyph_run(
    rect: Rect,
    text: String,
    color: Rgba8,
    style: TextStyle,
    opacity: u16,
) -> GlyphRun {
    let max_width = Some(rect.width.max(1.0));
    // The shared engine bootstraps with bundled FiraSans (G1.4 owned
    // engine). Layout failure is unreachable unless a future caller
    // hot-swaps in a malformed user font via FontCollection::register.
    let paragraph = TEXT_ENGINE.with(|engine| {
        engine
            .layout_paragraph_styled(&text, max_width, &style)
            .unwrap_or_else(|_| unreachable!("text engine layout failed unexpectedly"))
    });
    GlyphRun::new(rect, text, color, style, opacity).with_paragraph(paragraph)
}

thread_local! {
    static TEXT_ENGINE: OwnedTextEngine = OwnedTextEngine::new(TextConfig::default());
}

/// Resolve a node's border into a paint stroke (DR-057 D3). Width with
/// no explicit colour follows the CSS `currentColor` shape: the
/// resolved text colour, then the theme's on-surface. Inside alignment
/// matches the box model — the layout solvers already reserved the
/// border edge inside the rect.
fn border_stroke(style: &RenderStyle, theme: &Theme) -> Option<Stroke> {
    let width = style.border_width?;
    if width.0 <= 0 {
        return None;
    }
    let color = style
        .border_color
        .or(style.text_color)
        .unwrap_or(theme.on_surface);
    Some(Stroke {
        color,
        width: f32::from(width.0),
        style: StrokeStyle::Solid,
        align: StrokeAlign::Inside,
    })
}

fn text_content(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => value.clone(),
        TextContent::Reactive(value) => value.get(),
    }
}
