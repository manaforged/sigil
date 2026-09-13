use crate::hal::{Point, Rect};
use crate::style::Rgba8;

use super::*;
use crate::renderer::scene_command::{GradientStop, LinearGradient};
use crate::renderer::{CornerRadii, SceneGradientStyle, SceneRectStyle};

#[test]
fn capacity_counts_ordered_primitive_payloads() {
    let mut scene = crate::renderer::SceneBuilder::new();
    scene.draw_rect(rect(), SceneRectStyle::solid(color()));
    scene.draw_gradient(
        rect(),
        SceneGradientStyle::new(
            LinearGradient::new(
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                vec![
                    GradientStop::new(0.0, color()),
                    GradientStop::new(1.0, color()),
                ],
            ),
            CornerRadii::ZERO,
            10_000,
        ),
    );
    let scene = scene.finish();

    assert_eq!(
        UploadCapacities::from_commands(scene.commands.iter()),
        UploadCapacities {
            rects: 1,
            fills: 1,
            stops: 2,
            shadows: 0,
            decorations: 0,
            glyphs: 0,
        }
    );
}

const fn rect() -> Rect {
    Rect::new(Point::new(0.0, 0.0), 10.0, 10.0)
}

const fn color() -> Rgba8 {
    Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 255,
    }
}
