use crate::hal::{Color, Point};

use super::helpers::{rect, rgba};
use crate::renderer::scene_command::{GradientStop, LinearGradient};
use crate::renderer::{
    CommandScene, CornerRadii, LayerEffect, LayerScope, SceneBuilder, SceneGradientStyle,
    SceneRectStyle,
};
#[must_use]
pub fn glow() -> (CommandScene, (f32, f32), Color) {
    use crate::renderer::shadow_paint::Shadow;

    let fill = rgba(245, 245, 250, 255);
    let radii = CornerRadii::uniform(24.0);
    let mut scene = SceneBuilder::new();

    scene.draw_rect(
        rect(100.0, 80.0, 200.0, 200.0),
        SceneRectStyle::new(fill, radii, 10_000),
    );

    scene.draw_shadow(
        rect(380.0, 80.0, 200.0, 200.0),
        radii,
        Shadow::glow(rgba(80, 200, 255, 128), 8.0, 18.0),
    );
    scene.draw_rect(
        rect(380.0, 80.0, 200.0, 200.0),
        SceneRectStyle::new(fill, radii, 10_000),
    );

    scene.draw_shadow(
        rect(660.0, 80.0, 200.0, 200.0),
        radii,
        Shadow::glow(rgba(220, 80, 220, 180), 12.0, 28.0),
    );
    scene.draw_rect(
        rect(660.0, 80.0, 200.0, 200.0),
        SceneRectStyle::new(fill, radii, 10_000),
    );

    (
        scene.finish(),
        (960.0, 360.0),
        Color::rgba(0.05, 0.06, 0.09, 1.0),
    )
}

#[must_use]
pub fn layer_pool() -> (CommandScene, (f32, f32), Color) {
    let viewport = (960.0, 320.0);
    let clear = Color::rgba(0.05, 0.06, 0.09, 1.0);
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
    (scene.finish(), viewport, clear)
}
