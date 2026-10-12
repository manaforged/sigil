#![allow(clippy::shadow_unrelated)]

use crate::geometry::{Point, Rect};
use crate::style::Rgba8;

use super::*;
use crate::renderer::scene_command::{GradientStop, LinearGradient};
use crate::scene::{CornerRadii, SceneGradientStyle, SceneRectStyle};

#[test]
fn capacity_sees_primitive_payloads() {
    let rect = Rect::new(Point::new(0.0, 0.0), 10.0, 10.0);
    let color = Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 255,
    };
    assert!(!UploadCapacities::from_commands(core::iter::empty()).has_primitives);
    let mut scene = crate::scene::SceneBuilder::new();
    scene.draw_gradient(
        rect,
        SceneGradientStyle::new(
            LinearGradient::new(
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                vec![GradientStop::new(0.0, color), GradientStop::new(1.0, color)],
            ),
            CornerRadii::ZERO,
            10_000,
        ),
    );
    let scene = scene.finish();
    assert!(UploadCapacities::from_commands(scene.commands.iter()).has_primitives);
    let mut scene = crate::scene::SceneBuilder::new();
    scene.draw_rect(rect, SceneRectStyle::solid(color));
    assert!(UploadCapacities::from_commands(scene.finish().commands.iter()).has_primitives);
}
