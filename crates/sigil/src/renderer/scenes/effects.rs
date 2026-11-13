use crate::geometry::Point;
use crate::style::Color;
use crate::style::Rgba;

use super::helpers::{DARK, rect};
use crate::renderer::scene_command::{GradientStop, LinearGradient};
use crate::renderer::shadow_paint::Shadow;
use crate::scene::{
    CommandScene, CornerRadii, DrawGradient, DrawRect, LayerEffect, LayerScope, SceneBuilder,
};

pub fn glow() -> (CommandScene, (f32, f32), Rgba) {
    let fill = |rect: crate::geometry::Rect| {
        DrawRect::new(rect, Color::rgba(245, 245, 250, 255))
            .border_radius(CornerRadii::uniform(24.0))
    };
    let mut scene = SceneBuilder::new();
    scene.draw_rect(fill(rect(100.0, 80.0, 200.0, 200.0)));
    for (x, color, spread, blur) in [
        (380.0, Color::rgba(80, 200, 255, 128), 8.0, 18.0),
        (660.0, Color::rgba(220, 80, 220, 180), 12.0, 28.0),
    ] {
        let card = rect(x, 80.0, 200.0, 200.0);
        scene.draw_shadow(
            card,
            fill(card).border_radius.0,
            Shadow::glow(color, spread, blur),
        );
        scene.draw_rect(fill(card));
    }
    (scene.finish(), (960.0, 360.0), DARK)
}

pub fn layer_pool() -> (CommandScene, (f32, f32), Rgba) {
    let mut scene = SceneBuilder::new();
    scene.draw_gradient(DrawGradient::new(
        rect(0.0, 0.0, 960.0, 320.0),
        LinearGradient::new(
            Point::new(0.0, 0.0),
            Point::new(960.0, 0.0),
            vec![
                GradientStop::new(0.0, Color::rgba(220, 50, 50, 255)),
                GradientStop::new(1.0, Color::rgba(50, 200, 220, 255)),
            ],
        ),
    ));
    let fill = |rect: crate::geometry::Rect| {
        DrawRect::new(rect, Color::rgba(240, 240, 250, 255))
            .border_radius(CornerRadii::uniform(20.0))
    };
    scene.draw_rect(fill(rect(100.0, 80.0, 180.0, 160.0)));
    scene.push_layer(LayerScope {
        clip: rect(390.0, 80.0, 180.0, 160.0),
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::Identity,
        opacity: 0.5,
        blend: crate::scene::BlendMode::Normal,
    });
    scene.draw_rect(fill(rect(390.0, 80.0, 180.0, 160.0)));
    scene.pop_layer();
    scene.draw_rect(fill(rect(680.0, 80.0, 180.0, 160.0)));
    (scene.finish(), (960.0, 320.0), DARK)
}
