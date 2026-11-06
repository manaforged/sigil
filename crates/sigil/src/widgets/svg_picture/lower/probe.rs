use std::sync::Arc;

use resvg::usvg::Tree;

use crate::scene::command::CommandScene;
use crate::style::Rgba8;

use super::super::fallback::bitmap;
use super::lower;

/// Which path an SVG takes into a scene: one resvg bitmap or lowered scene commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SvgMode {
    /// One resvg bitmap, rastered again at the device size.
    Raster,
    /// Scene commands, with a bitmap only for subtrees that have no scene equivalent.
    Vector,
}

/// The scene that paints `tree` into a box of `size` at the origin over `surface`, and the fallbacks the lowering took.
pub fn svg_scene(
    tree: Arc<Tree>,
    key: u64,
    size: (f32, f32),
    surface: Rgba8,
    mode: SvgMode,
) -> (CommandScene, Vec<String>) {
    let backdrop = [surface.r, surface.g, surface.b];
    if mode == SvgMode::Vector {
        let lowered = lower(&tree, size, key, backdrop);
        let fallbacks = lowered.fallbacks.iter().map(ToString::to_string).collect();
        return (
            CommandScene {
                commands: lowered.commands,
            },
            fallbacks,
        );
    }
    (
        CommandScene {
            commands: bitmap(tree, size, key, backdrop),
        },
        Vec::new(),
    )
}
