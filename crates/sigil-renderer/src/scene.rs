//! Headless scene emission for renderer integration tests.

use sigil_core::{Cx, Node, NodeId, NodeType, TextContent};
use sigil_hal::Rect;
use sigil_layout::LayoutResult;
use sigil_style::{Px, Rgba8};

use crate::style_adapter::render_style_for;

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

    fn push(&mut self, item: SceneItem) {
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
    /// Optional corner radius.
    pub border_radius: Option<Px>,
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
    /// Opacity encoded as `value * 10000`.
    pub opacity: u16,
}

/// Emit a headless scene for a laid-out core subtree.
#[must_use]
pub fn emit_scene(cx: &Cx, root: NodeId) -> Scene {
    let mut scene = Scene::default();
    cx.visit_preorder(root, |id, node| push_node(&mut scene, id, node));
    scene
}

fn push_node(scene: &mut Scene, id: NodeId, node: &Node) {
    let style = render_style_for(id, node);
    let Some(layout) = node.get_extension::<LayoutResult>() else {
        return;
    };
    match &node.kind {
        NodeType::Container => {
            if let Some(color) = style.background_color {
                scene.push(SceneItem::Rect(SceneRect {
                    node: id,
                    rect: layout.rect(),
                    color,
                    border_radius: style.border_radius,
                    opacity: style.opacity,
                }));
            }
        }
        NodeType::Text { content } => {
            if let Some(color) = style.text_color {
                scene.push(SceneItem::Text(SceneText {
                    node: id,
                    rect: layout.rect(),
                    text: text_content(content),
                    color,
                    opacity: style.opacity,
                }));
            }
        }
        NodeType::Fragment => {}
    }
}

fn text_content(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => value.clone(),
        TextContent::Reactive(value) => value.get(),
    }
}
