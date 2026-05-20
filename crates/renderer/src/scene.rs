//! Headless scene emission for renderer integration tests.

use sigil_hal::{Point, Rect};
use sigil_kernel::{Cx, Node, NodeId, NodeType, TextContent};
use sigil_layout::LayoutResult;
use sigil_style::{DisplayKind, Rgba8, StyleAtom, StyleBinding, TextStyle};
use sigil_text::{OwnedTextEngine, TextConfig};

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
    push_subtree(&mut scene, cx, root);
    scene.finish()
}

fn push_subtree(scene: &mut SceneBuilder, cx: &Cx, id: NodeId) {
    let Some(node) = cx.get(id) else {
        return;
    };
    // Honour display:none — skip this node and its entire subtree, matching
    // CSS semantics where display:none removes the element from the render tree.
    if is_display_none(node) {
        return;
    }
    // `LayoutResult::rect()` is already in root coordinates — both
    // `BlockSolver` and `FlexSolver` accumulate every ancestor's
    // border + padding offsets into the `ComputedBox` they write.
    // Re-accumulating a parent_origin here would double-count every
    // ancestor offset and mislocate the node by the root-padding-and-
    // up sum. See `LayoutResult::rect` docs and the renderer's
    // `nested_text_scene_rect_is_absolute` regression test.
    let rect = node.get_extension::<LayoutResult>().map(LayoutResult::rect);
    if let Some(rect) = rect {
        push_focus_ring(scene, id, node, rect);
        push_node(scene, id, node, rect);
    }
    for child in node.children().iter().copied() {
        push_subtree(scene, cx, child);
    }
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

fn push_focus_ring(scene: &mut SceneBuilder, id: NodeId, node: &Node, rect: Rect) {
    let Some(ring) = focus_ring_for(node).filter(|ring| ring.width > 0.0) else {
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

fn push_node(scene: &mut SceneBuilder, id: NodeId, node: &Node, rect: Rect) {
    let style = render_style_for(id, node);
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
                        text_style_for(node),
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
