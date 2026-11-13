#![allow(clippy::cast_precision_loss)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::suboptimal_flops)]

use crate::geometry::Rect;
use crate::style::Color;
use crate::style::Rgba;

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
    DARK, ambient_shadow, draw_fill, draw_gradient_strokes, draw_solid_strokes, key_shadow,
    linear_gradient, rect, slash_kind, solid_stroke, stroke_with,
};

use crate::renderer::BUILTIN_SPRITE_ATLAS_ID;
use crate::renderer::sprite_atlas::{MaskShape, mask_origin};
use crate::scene::{
    CommandScene, CornerRadii, DecorationKind, DrawDecoration, DrawPattern, DrawRect, DrawSprite,
    PaintFill, PatternKind, SceneBuilder, Shadow, ShadowKind, SpriteKind, SpriteSource,
    StrokeAlign, StrokeStyle,
};

pub fn gradient() -> (CommandScene, (f32, f32), Rgba) {
    let viewport = (760.0, 320.0);
    let mut scene = SceneBuilder::new();
    scene.draw_rect(DrawRect::new(
        rect(0.0, 0.0, 760.0, 320.0),
        Color::rgba(7, 10, 15, 255),
    ));
    let white = |a: u8| Color::rgba(255, 255, 255, a);
    let fills = [
        linear_gradient(
            (120.0, 72.0),
            (120.0, 200.0),
            &[
                (0.0, Color::rgba(34, 86, 160, 255)),
                (1.0, Color::rgba(19, 28, 48, 255)),
            ],
        ),
        linear_gradient(
            (216.0, 136.0),
            (360.0, 136.0),
            &[
                (0.0, Color::rgba(99, 102, 241, 255)),
                (1.0, Color::rgba(14, 165, 233, 255)),
            ],
        ),
        linear_gradient(
            (384.0, 136.0),
            (528.0, 136.0),
            &[
                (0.0, Color::rgba(239, 68, 68, 255)),
                (0.34, Color::rgba(245, 158, 11, 255)),
                (0.67, Color::rgba(34, 197, 94, 255)),
                (1.0, Color::rgba(59, 130, 246, 255)),
            ],
        ),
        linear_gradient(
            (552.0, 72.0),
            (696.0, 200.0),
            &[(0.0, white(24)), (1.0, white(224))],
        ),
    ];
    for (x, fill) in [48.0, 216.0, 384.0, 552.0].into_iter().zip(fills) {
        draw_fill(&mut scene, rect(x, 72.0, 144.0, 128.0), fill);
    }
    (scene.finish(), viewport, Rgba::new(0.025, 0.03, 0.04, 1.0))
}

pub fn stroke() -> (CommandScene, (f32, f32), Rgba) {
    let mut scene = SceneBuilder::new();
    draw_solid_strokes(&mut scene);
    draw_gradient_strokes(&mut scene);
    let clear = Rgba::new(0.025, 0.03, 0.04, 1.0);
    (scene.finish(), (328.0, 192.0), clear)
}

pub fn corner_radius() -> (CommandScene, (f32, f32), Rgba) {
    let mut scene = SceneBuilder::new();
    for (index, (radii, color)) in [
        (CornerRadii::uniform(24.0), Color::rgba(38, 92, 180, 255)),
        (CornerRadii::top(24.0), Color::rgba(68, 156, 112, 255)),
        (CornerRadii::bottom(24.0), Color::rgba(210, 98, 82, 255)),
        (CornerRadii::left(24.0), Color::rgba(144, 86, 184, 255)),
        (CornerRadii::right(24.0), Color::rgba(205, 136, 46, 255)),
        (
            CornerRadii::new(40.0, 8.0, 40.0, 8.0),
            Color::rgba(52, 132, 152, 255),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        scene.draw_rect(
            DrawRect::new(
                rect(80.0 + index as f32 * 180.0, 130.0, 140.0, 140.0),
                color,
            )
            .border_radius(radii)
            .stroke(solid_stroke(4.0)),
        );
    }
    let clear = Rgba::new(0.11, 0.16, 0.22, 1.0);
    (scene.finish(), (1200.0, 400.0), clear)
}

pub fn shadow() -> (CommandScene, (f32, f32), Rgba) {
    let mut scene = SceneBuilder::new();
    let radii = CornerRadii::uniform(18.0);
    for (index, (elevation, color)) in [
        (0.0, Color::rgba(246, 248, 250, 255)),
        (1.0, Color::rgba(238, 245, 255, 255)),
        (3.0, Color::rgba(240, 250, 244, 255)),
        (6.0, Color::rgba(255, 244, 238, 255)),
        (8.0, Color::rgba(246, 242, 255, 255)),
        (12.0, Color::rgba(240, 248, 250, 255)),
    ]
    .into_iter()
    .enumerate()
    {
        let card = rect(80.0 + index as f32 * 180.0, 135.0, 130.0, 130.0);
        if elevation > 0.0 {
            scene.draw_shadow(card, radii.0, ambient_shadow(elevation));
            scene.draw_shadow(card, radii.0, key_shadow(elevation));
        }
        scene.draw_rect(DrawRect::new(card, color).border_radius(radii));
    }
    let clear = Rgba::new(0.90, 0.92, 0.94, 1.0);
    (scene.finish(), (1200.0, 400.0), clear)
}

pub fn inner_shadow() -> (CommandScene, (f32, f32), Rgba) {
    let mut scene = SceneBuilder::new();
    let radii = CornerRadii::uniform(18.0);
    for index in 0..5_u8 {
        let offset = f32::from(index);
        let field = rect(70.0 + offset * 220.0, 160.0, 200.0, 80.0);
        let press = Shadow::new([0.0, 2.0], 6.0, offset, [0.0, 0.0, 0.0, 0.45]);
        scene.draw_rect(DrawRect::new(field, Color::rgba(245, 247, 250, 255)).border_radius(radii));
        scene.draw_shadow(field, radii.0, press.kind(ShadowKind::Inner));
    }
    let clear = Rgba::new(0.88, 0.91, 0.94, 1.0);
    (scene.finish(), (1200.0, 400.0), clear)
}

pub fn pattern() -> (CommandScene, (f32, f32), Rgba) {
    let mut scene = SceneBuilder::new();
    let slash_rect = rect(60.0, 50.0, 540.0, 280.0);
    scene.draw_rect(
        DrawRect::new(slash_rect, Color::rgba(25, 34, 49, 255))
            .border_radius(CornerRadii::uniform(16.0)),
    );
    let slash = slash_kind(Color::rgba(255, 255, 255, 255), 40.0, 16.0);
    draw_fill(&mut scene, slash_rect, PaintFill::Pattern(slash));
    let checker = PatternKind::Checker {
        color_a: Color::rgba(220, 60, 60, 255),
        color_b: Color::rgba(245, 245, 250, 255),
        cell: 40.0,
    };
    draw_fill(
        &mut scene,
        rect(660.0, 50.0, 480.0, 280.0),
        PaintFill::Pattern(checker),
    );
    scene.draw_pattern(
        DrawPattern::new(
            rect(420.0, 245.0, 360.0, 90.0),
            slash_kind(Color::rgba(255, 255, 255, 220), 32.0, 10.0),
        )
        .border_radius(CornerRadii::top(24.0))
        .opacity(8_500)
        .stroke(stroke_with(StrokeStyle::Solid, StrokeAlign::Outside, 4.0)),
    );
    (scene.finish(), (1200.0, 380.0), DARK)
}

pub fn sprite() -> (CommandScene, (f32, f32), Rgba) {
    let mut scene = SceneBuilder::new();
    scene.draw_rect(DrawRect::new(
        rect(0.0, 0.0, 900.0, 360.0),
        Color::rgba(3, 4, 6, 255),
    ));
    for (source_x, phase, x, kind, tint) in [
        (
            0.0,
            0.0,
            80.0,
            SpriteKind::Mono,
            Color::rgba(220, 60, 60, 255),
        ),
        (
            200.0,
            0.35,
            360.0,
            SpriteKind::Subpixel,
            Color::rgba(28, 32, 44, 255),
        ),
        (
            400.0,
            0.0,
            640.0,
            SpriteKind::Poly,
            Color::rgba(80, 160, 240, 255),
        ),
    ] {
        scene.draw_sprite(DrawSprite::new(
            SpriteSource {
                atlas: BUILTIN_SPRITE_ATLAS_ID,
                region: rect(source_x, 0.0, 200.0, 200.0),
                phase,
            },
            rect(x, 80.0, 200.0, 200.0),
            kind,
            tint,
        ));
    }
    (scene.finish(), (900.0, 360.0), DARK)
}

pub fn icon() -> (CommandScene, (f32, f32), Rgba) {
    let mut scene = SceneBuilder::new();
    for (shape, dest, tint) in ICON_DRAWS {
        scene.draw_sprite(DrawSprite::new(
            icon_source(shape),
            dest,
            SpriteKind::Mono,
            tint,
        ));
    }
    (scene.finish(), (1024.0, 320.0), DARK)
}

const ICON_DRAWS: [(MaskShape, Rect, Color); 5] = [
    (
        MaskShape::Disc,
        rect(40.0, 80.0, 128.0, 128.0),
        Color::rgba(220, 80, 80, 255),
    ),
    (
        MaskShape::Ring,
        rect(208.0, 80.0, 128.0, 128.0),
        Color::rgba(200, 200, 220, 255),
    ),
    (
        MaskShape::Square,
        rect(376.0, 80.0, 128.0, 128.0),
        Color::rgba(100, 220, 140, 255),
    ),
    (
        MaskShape::Diamond,
        rect(544.0, 80.0, 128.0, 128.0),
        Color::rgba(230, 200, 100, 255),
    ),
    (
        MaskShape::Cross,
        rect(712.0, 80.0, 128.0, 128.0),
        Color::rgba(220, 100, 220, 255),
    ),
];

fn icon_source(shape: MaskShape) -> SpriteSource {
    let (x, y) = mask_origin(shape);
    SpriteSource {
        atlas: BUILTIN_SPRITE_ATLAS_ID,
        region: rect(x, y, 128.0, 128.0),
        phase: 0.0,
    }
}

pub fn decoration() -> (CommandScene, (f32, f32), Rgba) {
    let mut scene = SceneBuilder::new();
    for (kind, band, color, line_width, period) in [
        (
            DecorationKind::Underline,
            rect(80.0, 40.0, 720.0, 6.0),
            Color::rgba(100, 200, 255, 255),
            2.0,
            0.0,
        ),
        (
            DecorationKind::Strikethrough,
            rect(80.0, 100.0, 720.0, 4.0),
            Color::rgba(220, 60, 60, 255),
            2.0,
            0.0,
        ),
        (
            DecorationKind::WavyUnderline,
            rect(80.0, 160.0, 720.0, 16.0),
            Color::rgba(220, 140, 60, 255),
            2.5,
            32.0,
        ),
    ] {
        scene.draw_decoration(DrawDecoration {
            node: None,
            kind,
            band,
            color,
            opacity: 10_000,
            period,
            line_width,
        });
    }
    (scene.finish(), (800.0, 240.0), DARK)
}
