use crate::geometry::{Point, Rect};
use crate::style::{Theme, Token};

use super::{CommandScene, DrawRect, SceneCommand, SceneRectStyle};

pub fn paint_backdrop(scene: &mut CommandScene, theme: &Theme, viewport: (f32, f32)) {
    let rect = Rect::new(Point::new(0.0, 0.0), viewport.0, viewport.1);
    scene.commands.insert(
        0,
        SceneCommand::Rect(DrawRect::new(
            rect,
            SceneRectStyle::solid(theme.color(Token::Surface)),
        )),
    );
}
