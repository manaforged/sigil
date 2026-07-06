//! Headless scene emission for renderer integration tests.

use hal::{Point, Rect};
use kernel::{Cx, Node, NodeId, NodeType, TextContent};
use layout::LayoutResult;
use style::{DisplayKind, RenderStyle, Rgba8, StyleAtom, StyleBinding, TextStyle, Theme};
use text::{OwnedTextEngine, TextConfig};

use crate::corner_radii::CornerRadii;
use crate::scene_builder::SceneBuilder;
use crate::scene_command::{CommandScene, GlyphRun, SceneRectStyle};
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
    };
    push_subtree(&mut scene, cx, root, inherited);
    scene.finish()
}

/// Style context cascading down the scene walk: the effective theme plus
/// the nearest ancestor's resolved text color. The color half is the
/// CSS-style `color` cascade — a container's `text-*` class (static or
/// reactive `class_when`) colors every descendant text node that doesn't
/// set its own.
#[derive(Clone, Copy)]
struct Inherited<'t> {
    theme: &'t Theme,
    text_color: Option<Rgba8>,
}

fn push_subtree(scene: &mut SceneBuilder, cx: &Cx, id: NodeId, inherited: Inherited<'_>) {
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
    let child_inherited = Inherited {
        theme,
        text_color: style.text_color.or(inherited.text_color),
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
            },
        );
        // `overflow: hidden`/`scroll` → scissor descendants to this node's box, so a
        // too-tall child (a long file in the editor pane, scrolled-out content) is
        // cut at the edge instead of painting over its siblings. The command walker
        // skips any glyph run / fill whose rect falls outside the active clip.
        if result.clips() {
            scene.with_clip(rect, |scene| {
                push_children(scene, cx, node, child_inherited);
            });
            return;
        }
    }
    push_children(scene, cx, node, child_inherited);
}

fn push_children(scene: &mut SceneBuilder, cx: &Cx, node: &Node, inherited: Inherited<'_>) {
    for child in node.children().iter().copied() {
        push_subtree(scene, cx, child, inherited);
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
        || binding.reactive_slice().contains(&target)
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

/// The resolved paint inputs for one node: its render style plus the
/// effective theme (still needed for glyph-run text styling).
struct NodePaint<'a> {
    style: &'a RenderStyle,
    theme: &'a Theme,
}

fn push_node(scene: &mut SceneBuilder, id: NodeId, node: &Node, rect: Rect, paint: &NodePaint<'_>) {
    let style = paint.style;
    match &node.kind {
        NodeType::Container => {
            if let Some(color) = style.background_color {
                scene.draw_rect_for_node(
                    id,
                    rect,
                    SceneRectStyle::new(
                        color,
                        corner_radii_for(style.border_radius),
                        style.opacity,
                    ),
                );
            }
        }
        NodeType::Text { content } => {
            if let Some(color) = style.text_color {
                scene.draw_glyph_run_for_node(
                    id,
                    shape_glyph_run(
                        rect,
                        text_content(content),
                        color,
                        text_style_for(node, paint.theme),
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

fn text_content(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => value.clone(),
        TextContent::Reactive(value) => value.get(),
    }
}
