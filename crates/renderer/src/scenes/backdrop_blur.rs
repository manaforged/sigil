use sigil_hal::{Color, Rect};

use super::helpers::{rect, rgba};
use crate::{CommandScene, CornerRadii, LayerEffect, LayerScope, SceneBuilder, SceneRectStyle};

#[must_use]
pub fn backdrop_blur() -> (CommandScene, (f32, f32), Color) {
    let viewport = (1024.0, 360.0);
    let clear = Color::rgba(0.05, 0.06, 0.09, 1.0);
    let mut scene = SceneBuilder::new();

    draw_bar_pattern(&mut scene, rect(0.0, 0.0, viewport.0, viewport.1));

    push_blur_surface(&mut scene, rect(100.0, 60.0, 240.0, 240.0), 0.0);
    push_blur_surface(&mut scene, rect(400.0, 60.0, 240.0, 240.0), 6.0);
    push_blur_surface(&mut scene, rect(700.0, 60.0, 240.0, 240.0), 18.0);

    (scene.finish(), viewport, clear)
}

fn push_blur_surface(scene: &mut SceneBuilder, clip: Rect, sigma: f32) {
    scene.push_layer(LayerScope {
        clip,
        clip_radii: CornerRadii::ZERO,
        effect: LayerEffect::LayerBlur { sigma },
        opacity: 1.0,
    });
    draw_bar_pattern(scene, clip);
    scene.pop_layer();
}

fn draw_bar_pattern(scene: &mut SceneBuilder, clip: Rect) {
    scene.draw_rect(clip, SceneRectStyle::solid(rgba(255, 255, 255, 255)));
    let right_edge = clip.origin.x + clip.width;
    for k in 0..64_u32 {
        let x = 4.0 + (k * 16) as f32;
        if x >= right_edge {
            break;
        }
        let bar_right = (x + 8.0).min(right_edge);
        scene.draw_rect(
            rect(x, clip.origin.y, bar_right - x, clip.height),
            SceneRectStyle::solid(rgba(0, 0, 0, 255)),
        );
    }
}
