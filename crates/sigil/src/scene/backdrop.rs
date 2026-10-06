use crate::geometry::{Point, Rect};
use crate::style::{Theme, Token};

use super::{CommandScene, DrawRect, SceneCommand, SceneRectStyle};

const OVERSCAN: f32 = 1.0;

pub fn paint_backdrop(scene: &mut CommandScene, theme: &Theme, viewport: (f32, f32)) {
    let rect = Rect::new(
        Point::new(-OVERSCAN, -OVERSCAN),
        viewport.0 + OVERSCAN + OVERSCAN,
        viewport.1 + OVERSCAN + OVERSCAN,
    );
    scene.commands.insert(
        0,
        SceneCommand::Rect(DrawRect::new(
            rect,
            SceneRectStyle::solid(theme.color(Token::Surface)),
        )),
    );
}
