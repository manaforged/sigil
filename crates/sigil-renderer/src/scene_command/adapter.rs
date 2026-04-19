// Adapter from the legacy flat scene to command-buffer scenes.

use crate::scene::{Scene, SceneItem, shape_glyph_run};
use crate::scene_builder::SceneBuilder;
use crate::scene_command::{CommandScene, SceneRectStyle};

#[allow(dead_code, clippy::redundant_pub_crate)]
pub(crate) fn flat_to_commands(scene: &Scene) -> CommandScene {
    let mut builder = SceneBuilder::new();
    for item in scene.items() {
        match item {
            SceneItem::Rect(rect) => builder.draw_rect_for_node(
                rect.node,
                rect.rect,
                SceneRectStyle::new(rect.color, rect.border_radius, rect.opacity),
            ),
            SceneItem::Text(text) => builder.draw_glyph_run_for_node(
                text.node,
                shape_glyph_run(
                    text.rect,
                    text.text.clone(),
                    text.color,
                    text.style.clone(),
                    text.opacity,
                ),
            ),
        }
    }
    builder.finish()
}
