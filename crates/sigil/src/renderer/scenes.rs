#![allow(clippy::cast_precision_loss)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::suboptimal_flops)]

use std::sync::OnceLock;

use crate::geometry::Rect;
use crate::icons::{IconBaker, builtin};
use crate::style::Color;
use crate::style::Rgba8;

mod backdrop_blur;
mod effects;
mod helpers;
mod text;
mod translucent;

pub use backdrop_blur::backdrop_blur;
pub use effects::{glow, layer_pool};
pub use text::text;
pub use translucent::translucent;

use helpers::{
    alpha_overlay, ambient_shadow, card_color, card_rect, checker_pattern, corner_color_for,
    draw_fill, draw_gradient_strokes, draw_solid_strokes, field_color, field_rect,
    inner_press_shadow, key_shadow, rainbow, rect, rgba, slash_kind, slash_pattern, solid_stroke,
    sprite_source, stripe_horizontal, stroke_with, subtle_vertical,
};

use crate::scene::{
    CommandScene, CornerRadii, DecorationKind, SceneBuilder, SceneDecorationStyle,
    ScenePatternStyle, SceneRectStyle, SpriteKind, SpriteSource, StrokeAlign, StrokeStyle,
};

pub use crate::renderer::sprite_atlas::BUILTIN_ICON_ATLAS_BASE_ID;

static BUILTIN_ICONS: OnceLock<Vec<crate::icons::BakedIcon>> = OnceLock::new();

#[must_use]
pub fn gradient() -> (CommandScene, (f32, f32), Color) {
    let viewport = (760.0, 320.0);
    let mut scene = SceneBuilder::new();
    scene.draw_rect(
        rect(0.0, 0.0, 760.0, 320.0),
        SceneRectStyle::new(rgba(7, 10, 15, 255), CornerRadii::ZERO, 10_000),
    );
    draw_fill(
        &mut scene,
        rect(48.0, 72.0, 144.0, 128.0),
        subtle_vertical(),
    );
    draw_fill(
        &mut scene,
        rect(216.0, 72.0, 144.0, 128.0),
        stripe_horizontal(),
    );
    draw_fill(&mut scene, rect(384.0, 72.0, 144.0, 128.0), rainbow());
    draw_fill(&mut scene, rect(552.0, 72.0, 144.0, 128.0), alpha_overlay());
    (
        scene.finish(),
        viewport,
        Color::rgba(0.025, 0.03, 0.04, 1.0),
    )
}

#[must_use]
pub fn stroke() -> (CommandScene, (f32, f32), Color) {
    let mut scene = SceneBuilder::new();
    draw_solid_strokes(&mut scene);
    draw_gradient_strokes(&mut scene);
    (
        scene.finish(),
        (328.0, 192.0),
        Color::rgba(0.025, 0.03, 0.04, 1.0),
    )
}

#[must_use]
pub fn corner_radius() -> (CommandScene, (f32, f32), Color) {
    let mut scene = SceneBuilder::new();
    for (index, radii) in [
        CornerRadii::uniform(24.0),
        CornerRadii::top(24.0),
        CornerRadii::bottom(24.0),
        CornerRadii::left(24.0),
        CornerRadii::right(24.0),
        CornerRadii::new(40.0, 8.0, 40.0, 8.0),
    ]
    .into_iter()
    .enumerate()
    {
        scene.draw_rect(
            rect(80.0 + index as f32 * 180.0, 130.0, 140.0, 140.0),
            SceneRectStyle::new(corner_color_for(index), radii, 10_000)
                .with_stroke(solid_stroke(4.0)),
        );
    }
    (
        scene.finish(),
        (1200.0, 400.0),
        Color::rgba(0.11, 0.16, 0.22, 1.0),
    )
}

#[must_use]
pub fn shadow() -> (CommandScene, (f32, f32), Color) {
    let mut scene = SceneBuilder::new();
    for (index, elevation) in [0.0, 1.0, 3.0, 6.0, 8.0, 12.0].into_iter().enumerate() {
        let rect = card_rect(index);
        let radii = CornerRadii::uniform(18.0);
        if elevation > 0.0 {
            scene.draw_shadow(rect, radii, ambient_shadow(elevation));
            scene.draw_shadow(rect, radii, key_shadow(elevation));
        }
        scene.draw_rect(rect, SceneRectStyle::new(card_color(index), radii, 10_000));
    }
    (
        scene.finish(),
        (1200.0, 400.0),
        Color::rgba(0.90, 0.92, 0.94, 1.0),
    )
}

#[must_use]
pub fn inner_shadow() -> (CommandScene, (f32, f32), Color) {
    let mut scene = SceneBuilder::new();
    for index in 0..5 {
        let rect = field_rect(index);
        let radii = CornerRadii::uniform(18.0);
        scene.draw_rect(rect, SceneRectStyle::new(field_color(index), radii, 10_000));
        scene.draw_shadow(rect, radii, inner_press_shadow(index));
    }
    (
        scene.finish(),
        (1200.0, 400.0),
        Color::rgba(0.88, 0.91, 0.94, 1.0),
    )
}

#[must_use]
pub fn pattern() -> (CommandScene, (f32, f32), Color) {
    let mut scene = SceneBuilder::new();
    let slash_rect = rect(60.0, 50.0, 540.0, 280.0);
    scene.draw_rect(
        slash_rect,
        SceneRectStyle::new(rgba(25, 34, 49, 255), CornerRadii::uniform(16.0), 10_000),
    );
    draw_fill(&mut scene, slash_rect, slash_pattern());
    draw_fill(
        &mut scene,
        rect(660.0, 50.0, 480.0, 280.0),
        checker_pattern(),
    );
    scene.draw_pattern(
        rect(420.0, 245.0, 360.0, 90.0),
        ScenePatternStyle::new(
            slash_kind(rgba(255, 255, 255, 220), 32.0, 10.0),
            CornerRadii::top(24.0),
            8_500,
        )
        .with_stroke(stroke_with(StrokeStyle::Solid, StrokeAlign::Outside, 4.0)),
    );
    (
        scene.finish(),
        (1200.0, 380.0),
        Color::rgba(0.05, 0.06, 0.09, 1.0),
    )
}

#[must_use]
pub fn sprite() -> (CommandScene, (f32, f32), Color) {
    let mut scene = SceneBuilder::new();
    scene.draw_rect(
        rect(0.0, 0.0, 900.0, 360.0),
        SceneRectStyle::new(rgba(3, 4, 6, 255), CornerRadii::ZERO, 10_000),
    );
    scene.draw_sprite(
        sprite_source(0.0, 0.0, 0.0),
        rect(80.0, 80.0, 200.0, 200.0),
        SpriteKind::Mono,
        rgba(220, 60, 60, 255),
    );
    scene.draw_sprite(
        sprite_source(200.0, 0.0, 0.35),
        rect(360.0, 80.0, 200.0, 200.0),
        SpriteKind::Subpixel,
        rgba(28, 32, 44, 255),
    );
    scene.draw_sprite(
        sprite_source(400.0, 0.0, 0.0),
        rect(640.0, 80.0, 200.0, 200.0),
        SpriteKind::Poly,
        rgba(80, 160, 240, 255),
    );
    (
        scene.finish(),
        (900.0, 360.0),
        Color::rgba(0.05, 0.06, 0.09, 1.0),
    )
}

#[must_use]
pub fn icon() -> (CommandScene, (f32, f32), Color) {
    let _ = BUILTIN_ICONS.get_or_init(bake_builtin_icons);
    let mut scene = SceneBuilder::new();
    for (index, (dest, tint)) in ICON_DRAWS.into_iter().enumerate() {
        scene.draw_sprite(icon_source(index), dest, SpriteKind::Mono, tint);
    }
    (
        scene.finish(),
        (1024.0, 320.0),
        Color::rgba(0.05, 0.06, 0.09, 1.0),
    )
}

#[must_use]
pub fn decoration() -> (CommandScene, (f32, f32), Color) {
    let mut scene = SceneBuilder::new();
    scene.draw_decoration(decoration_style(
        DecorationKind::Underline,
        rect(80.0, 40.0, 720.0, 6.0),
        rgba(100, 200, 255, 255),
        2.0,
        0.0,
    ));
    scene.draw_decoration(decoration_style(
        DecorationKind::Strikethrough,
        rect(80.0, 100.0, 720.0, 4.0),
        rgba(220, 60, 60, 255),
        2.0,
        0.0,
    ));
    scene.draw_decoration(decoration_style(
        DecorationKind::WavyUnderline,
        rect(80.0, 160.0, 720.0, 16.0),
        rgba(220, 140, 60, 255),
        2.5,
        32.0,
    ));
    (
        scene.finish(),
        (800.0, 240.0),
        Color::rgba(0.05, 0.06, 0.09, 1.0),
    )
}

const ICON_DRAWS: [(Rect, Rgba8); 5] = [
    (rect(40.0, 80.0, 128.0, 128.0), rgba(220, 80, 80, 255)),
    (rect(208.0, 80.0, 128.0, 128.0), rgba(200, 200, 220, 255)),
    (rect(376.0, 80.0, 128.0, 128.0), rgba(100, 220, 140, 255)),
    (rect(544.0, 80.0, 128.0, 128.0), rgba(230, 200, 100, 255)),
    (rect(712.0, 80.0, 128.0, 128.0), rgba(220, 100, 220, 255)),
];

fn icon_source(index: usize) -> SpriteSource {
    let atlas = ICON_ATLAS_IDS
        .get(index)
        .copied()
        .unwrap_or(BUILTIN_ICON_ATLAS_BASE_ID);
    SpriteSource {
        atlas,
        region: rect(0.0, 0.0, 128.0, 128.0),
        phase: 0.0,
    }
}

const ICON_ATLAS_IDS: [u32; 5] = [
    BUILTIN_ICON_ATLAS_BASE_ID,
    BUILTIN_ICON_ATLAS_BASE_ID + 1,
    BUILTIN_ICON_ATLAS_BASE_ID + 2,
    BUILTIN_ICON_ATLAS_BASE_ID + 3,
    BUILTIN_ICON_ATLAS_BASE_ID + 4,
];

fn bake_builtin_icons() -> Vec<crate::icons::BakedIcon> {
    let baker = IconBaker::new();
    builtin::ALL
        .iter()
        .map(|(_, svg)| {
            baker
                .bake(svg, 128, 128)
                .unwrap_or_else(|_| crate::icons::BakedIcon {
                    width: 128,
                    height: 128,
                    alpha: vec![0; 128 * 128],
                })
        })
        .collect()
}

const fn decoration_style(
    kind: DecorationKind,
    band: Rect,
    color: Rgba8,
    line_width: f32,
    period: f32,
) -> SceneDecorationStyle {
    SceneDecorationStyle {
        kind,
        band,
        color,
        opacity: 10_000,
        period,
        line_width,
    }
}
