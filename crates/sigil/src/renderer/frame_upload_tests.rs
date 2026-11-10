#![allow(clippy::shadow_unrelated)]

use crate::geometry::{Point, Rect};
use crate::style::Color;

use super::*;
use crate::renderer::scene_command::{GradientStop, LinearGradient};
use crate::scene::{DrawGradient, DrawRect};

#[test]
fn capacity_sees_primitive_payloads() {
    let rect = Rect::new(Point::new(0.0, 0.0), 10.0, 10.0);
    let color = Color {
        r: 1,
        g: 2,
        b: 3,
        a: 255,
    };
    assert!(!UploadCapacities::from_commands(core::iter::empty()).has_primitives);
    let mut scene = crate::scene::SceneBuilder::new();
    scene.draw_gradient(DrawGradient::new(
        rect,
        LinearGradient::new(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            vec![GradientStop::new(0.0, color), GradientStop::new(1.0, color)],
        ),
    ));
    let scene = scene.finish();
    assert!(UploadCapacities::from_commands(scene.commands.iter()).has_primitives);
    let mut scene = crate::scene::SceneBuilder::new();
    scene.draw_rect(DrawRect::new(rect, color));
    assert!(UploadCapacities::from_commands(scene.finish().commands.iter()).has_primitives);
}
