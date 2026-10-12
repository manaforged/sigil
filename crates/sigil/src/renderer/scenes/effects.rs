use crate::geometry::Point;
use crate::style::Color;

use super::helpers::{DARK, rect, rgba};
use crate::renderer::scene_command::{GradientStop, LinearGradient};
use crate::renderer::shadow_paint::Shadow;
use crate::scene::{
    CommandScene, CornerRadii, LayerEffect, LayerScope, SceneBuilder, SceneGradientStyle,
    SceneRectStyle,
};

#[must_use]
pub fn glow() -> (CommandScene, (f32, f32), Color) {
    let fill = SceneRectStyle::new(rgba(245, 245, 250, 255), CornerRadii::uniform(24.0), 10_000);
    let mut scene = SceneBuilder::new();
    scene.draw_rect(rect(100.0, 80.0, 200.0, 200.0), fill);
    for (x, color, spread, blur) in [
        (380.0, rgba(80, 200, 255, 128), 8.0, 18.0),
        (660.0, rgba(220, 80, 220, 180), 12.0, 28.0),
    ] {
        let card = rect(x, 80.0, 200.0, 200.0);
        scene.draw_shadow(card, fill.border_radius, Shadow::glow(color, spread, blur));
        scene.draw_rect(card, fill);
    }
    (scene.finish(), (960.0, 360.0), DARK)
}

#[must_use]
pub fn layer_pool() -> (CommandScene, (f32, f32), Color) {
    let mut scene = SceneBuilder::new();
    scene.draw_gradient(
        rect(0.0, 0.0, 960.0, 320.0),
        SceneGradientStyle::new(
            LinearGradient::new(
                Point::new(0.0, 0.0),
                Point::new(960.0, 0.0),
                vec![
                    GradientStop::new(0.0, rgba(220, 50, 50, 255)),
                    GradientStop::new(1.0, rgba(50, 200, 220, 255)),
                ],
            ),
            CornerRadii::ZERO,
            10_000,
        ),
    );
    let fill = SceneRectStyle::new(rgba(240, 240, 250, 255), CornerRadii::uniform(20.0), 10_000);
    scene.draw_rect(rect(100.0, 80.0, 180.0, 160.0), fill);
    scene.push_layer(LayerScope {
        clip: rect(390.0, 80.0, 180.0, 160.0),
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::Identity,
        opacity: 0.5,
    });
    scene.draw_rect(rect(390.0, 80.0, 180.0, 160.0), fill);
    scene.pop_layer();
    scene.draw_rect(rect(680.0, 80.0, 180.0, 160.0), fill);
    (scene.finish(), (960.0, 320.0), DARK)
}
