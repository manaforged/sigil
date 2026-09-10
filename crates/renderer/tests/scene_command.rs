#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil_hal::{Point, Rect};
use sigil_renderer::{CornerRadii, SceneBuilder, SceneCommand, SceneRectStyle, Transform};
use sigil_style::Rgba8;

#[test]
fn builder_emits_balanced_push_pop_for_with_clip() {
    let mut builder = SceneBuilder::new();

    builder.with_clip(rect(0.0, 0.0, 100.0, 80.0), |scene| {
        scene.draw_rect(rect(4.0, 6.0, 20.0, 12.0), rect_style());
    });

    let scene = builder.finish();
    let kinds = command_kinds(&scene.commands);
    assert_eq!(kinds.as_slice(), ["PushClip", "Rect", "PopClip"]);
}

#[test]
fn builder_nests_layers_correctly() {
    let mut builder = SceneBuilder::new();

    builder.with_layer(0.8, |scene| {
        scene.draw_rect(rect(0.0, 0.0, 10.0, 10.0), rect_style());
        scene.with_layer(0.5, |scene| {
            scene.draw_rect(rect(1.0, 1.0, 8.0, 8.0), rect_style());
        });
    });

    let scene = builder.finish();
    let kinds = command_kinds(&scene.commands);
    assert_eq!(
        kinds.as_slice(),
        [
            "SaveLayer",
            "Rect",
            "SaveLayer",
            "Rect",
            "RestoreLayer",
            "RestoreLayer",
        ]
    );
    let opacities = save_layer_opacities(&scene.commands);
    match opacities.as_slice() {
        [outer, inner] => {
            assert!(approx_eq(*outer, 0.8));
            assert!(approx_eq(*inner, 0.5));
        }
        other => assert_eq!(other.len(), 2),
    }
}

#[test]
fn builder_closes_scopes_on_drop_of_inner_scope() {
    let mut builder = SceneBuilder::new();
    let transform: Transform = [[1.0, 0.0, 4.0], [0.0, 1.0, 8.0]];

    builder.with_clip(rect(0.0, 0.0, 100.0, 80.0), |scene| {
        scene.with_transform(transform, |scene| {
            scene.draw_rect(rect(2.0, 2.0, 12.0, 12.0), rect_style());
        });
        scene.draw_rect(rect(20.0, 20.0, 12.0, 12.0), rect_style());
    });

    let scene = builder.finish();
    let kinds = command_kinds(&scene.commands);
    assert_eq!(
        kinds.as_slice(),
        [
            "PushClip",
            "PushTransform",
            "Rect",
            "PopTransform",
            "Rect",
            "PopClip",
        ]
    );
}

#[test]
fn builder_draw_rect_outside_any_scope_emits_only_draw() {
    let mut builder = SceneBuilder::new();
    let expected_rect = rect(4.0, 8.0, 16.0, 32.0);
    let expected_style = rect_style();

    builder.draw_rect(expected_rect, expected_style);

    let scene = builder.finish();
    let kinds = command_kinds(&scene.commands);
    assert_eq!(kinds.as_slice(), ["Rect"]);
    match scene.commands.as_slice() {
        [SceneCommand::Rect(draw)] => {
            assert!(same_rect(draw.rect, expected_rect));
            assert_eq!(draw.color, expected_style.color);
            assert_eq!(draw.border_radius, expected_style.border_radius);
            assert_eq!(draw.opacity, expected_style.opacity);
        }
        other => {
            let other_kinds = command_kinds(other);
            assert_eq!(other_kinds.as_slice(), ["Rect"]);
        }
    }
}

#[test]
fn command_scene_clone_is_deep() {
    let mut builder = SceneBuilder::new();
    builder.draw_rect(rect(0.0, 0.0, 10.0, 10.0), rect_style());

    let scene = builder.finish();
    let cloned = scene.clone();
    let original_kinds = command_kinds(&scene.commands);
    let cloned_kinds = command_kinds(&cloned.commands);

    assert_eq!(original_kinds, cloned_kinds);
    assert_ne!(scene.commands.as_ptr(), cloned.commands.as_ptr());
}

const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), width, height)
}

const fn rect_style() -> SceneRectStyle {
    SceneRectStyle::new(
        Rgba8 {
            r: 20,
            g: 40,
            b: 80,
            a: 255,
        },
        CornerRadii::uniform(6.0),
        9_500,
    )
}

fn command_kinds(commands: &[SceneCommand]) -> Vec<&'static str> {
    commands
        .iter()
        .map(|command| match command {
            SceneCommand::PushClip(_) => "PushClip",
            SceneCommand::PopClip => "PopClip",
            SceneCommand::PushTransform(_) => "PushTransform",
            SceneCommand::PopTransform => "PopTransform",
            SceneCommand::SaveLayer { .. } => "SaveLayer",
            SceneCommand::RestoreLayer => "RestoreLayer",
            SceneCommand::PushLayer(_) => "PushLayer",
            SceneCommand::PopLayer => "PopLayer",
            SceneCommand::Rect(_) => "Rect",
            SceneCommand::Gradient(_) => "Gradient",
            SceneCommand::Pattern(_) => "Pattern",
            SceneCommand::Sprite(_) => "Sprite",
            SceneCommand::Image(_) => "Image",
            SceneCommand::Decoration(_) => "Decoration",
            SceneCommand::DrawShadow { .. } => "DrawShadow",
            SceneCommand::GlyphRun(_) => "GlyphRun",
        })
        .collect()
}

fn save_layer_opacities(commands: &[SceneCommand]) -> Vec<f32> {
    commands
        .iter()
        .filter_map(|command| match command {
            SceneCommand::SaveLayer { opacity } => Some(*opacity),
            _ => None,
        })
        .collect()
}

fn same_rect(left: Rect, right: Rect) -> bool {
    approx_eq(left.origin.x, right.origin.x)
        && approx_eq(left.origin.y, right.origin.y)
        && approx_eq(left.width, right.width)
        && approx_eq(left.height, right.height)
}

fn approx_eq(left: f32, right: f32) -> bool {
    (left - right).abs() <= f32::EPSILON
}
