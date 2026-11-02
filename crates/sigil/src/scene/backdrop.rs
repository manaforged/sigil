use crate::geometry::{Point, Rect};
use crate::style::{Theme, Token};

use super::{CommandScene, DrawRect, SceneCommand};

const OVERSCAN: f32 = 1.0;

/// Put a rectangle of the theme's `Token::Surface` color under every command in `scene`, as an opaque window paints behind its tree.
pub fn paint_backdrop(scene: &mut CommandScene, theme: &Theme, viewport: (f32, f32)) {
    let rect = Rect::new(
        Point::new(-OVERSCAN, -OVERSCAN),
        viewport.0 + OVERSCAN + OVERSCAN,
        viewport.1 + OVERSCAN + OVERSCAN,
    );
    scene.commands.insert(
        0,
        SceneCommand::Rect(DrawRect::new(rect, theme.color(Token::Surface))),
    );
}
