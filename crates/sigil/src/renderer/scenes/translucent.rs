use smallvec::smallvec;

use crate::hal::Color;
use crate::scene::{CommandScene, CornerRadii, GradientStop, SceneBuilder, SceneRectStyle};

use super::helpers::{draw_fill, linear_gradient, rect, rgba};

#[must_use]
pub fn translucent() -> (CommandScene, (f32, f32), Color) {
    let mut scene = SceneBuilder::new();
    scene.draw_rect(
        rect(0.0, 0.0, 760.0, 320.0),
        SceneRectStyle::new(rgba(40, 90, 150, 255), CornerRadii::ZERO, 10_000),
    );
    for (x, y, color) in [
        (48.0, 48.0, rgba(230, 60, 60, 128)),
        (148.0, 108.0, rgba(60, 220, 90, 128)),
        (248.0, 48.0, rgba(250, 230, 60, 96)),
    ] {
        scene.draw_rect(
            rect(x, y, 200.0, 160.0),
            SceneRectStyle::new(color, CornerRadii::uniform(12.0), 10_000),
        );
    }
    draw_fill(
        &mut scene,
        rect(488.0, 48.0, 224.0, 224.0),
        linear_gradient(
            (488.0, 48.0),
            (712.0, 272.0),
            smallvec![
                GradientStop::new(0.0, rgba(255, 120, 200, 64)),
                GradientStop::new(1.0, rgba(255, 255, 255, 192)),
            ],
        ),
    );
    (
        scene.finish(),
        (760.0, 320.0),
        Color::rgba(0.02, 0.1, 0.3, 1.0),
    )
}
