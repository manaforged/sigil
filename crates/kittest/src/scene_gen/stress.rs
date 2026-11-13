use sigil::geometry::{Point, Rect};
use sigil::kernel::{ImageSource, ObjectFit};
use sigil::scene::{
    CornerRadii, DrawImage, DrawRect, LayerEffect, LayerScope, SceneCommand, shape_glyph_run,
};
use sigil::style::{Color, Rgba, TextStyle};

use super::GeneratedScene;

const INK: Color = Color {
    r: 230,
    g: 120,
    b: 40,
    a: 255,
};

const fn scene(commands: Vec<SceneCommand>, viewport: (f32, f32), scale: f32) -> GeneratedScene {
    GeneratedScene {
        commands,
        viewport,
        scale,
        clear: Rgba::new(0.05, 0.06, 0.09, 1.0),
    }
}

const fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(Point::new(x, y), w, h)
}

const fn fill(area: Rect) -> SceneCommand {
    SceneCommand::Rect(DrawRect::new(area, INK))
}

/// Builds a 128 by 128 scene with one filled rectangle inside `depth` nested pairs of a clip and a transform.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn clip_depth(depth: usize) -> GeneratedScene {
    let mut commands = Vec::with_capacity(depth * 2 + 1);
    for i in 0..depth {
        let inset = (i as f32) * 0.05;
        commands.push(SceneCommand::PushClip(
            rect(inset, inset, 128.0 - inset, 128.0 - inset).into(),
        ));
        commands.push(SceneCommand::PushTransform([
            [1.0, 0.0, 0.01],
            [0.0, 1.0, 0.0],
        ]));
    }
    commands.push(fill(rect(0.0, 0.0, 128.0, 128.0)));
    for _ in 0..depth {
        commands.push(SceneCommand::PopTransform);
        commands.push(SceneCommand::PopClip);
    }
    scene(commands, (128.0, 128.0), 1.0)
}

/// Builds a 128 by 128 scene with one filled rectangle inside `depth` nested pairs of a layer and a saved layer, where each layer has a blur filter of 1.0 when `blur` is true.
#[must_use]
pub fn layer_depth(depth: usize, blur: bool) -> GeneratedScene {
    let mut commands = Vec::with_capacity(depth * 2 + 1);
    for _ in 0..depth {
        let effect = if blur {
            LayerEffect::Filter(std::sync::Arc::new([sigil::scene::Filter::Blur(1.0)]))
        } else {
            LayerEffect::Identity
        };
        commands.push(SceneCommand::PushLayer(LayerScope {
            clip: rect(0.0, 0.0, 128.0, 128.0),
            clip_radii: CornerRadii::uniform(4.0),
            effect,
            opacity: 0.99,
            blend: sigil::scene::BlendMode::Normal,
        }));
        commands.push(SceneCommand::SaveLayer { opacity: 0.99 });
    }
    commands.push(fill(rect(16.0, 16.0, 96.0, 96.0)));
    for _ in 0..depth {
        commands.push(SceneCommand::RestoreLayer);
        commands.push(SceneCommand::PopLayer);
    }
    scene(commands, (128.0, 128.0), 1.0)
}

/// Builds a 1024 by 512 scene of text rows of 64 consecutive characters from U+4E00, with enough rows to cover `count` characters.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn unique_glyphs(count: u32) -> GeneratedScene {
    let mut commands = Vec::new();
    let per_row = 64_u32;
    for row in 0..count.div_ceil(per_row) {
        let text: String = (0..per_row)
            .filter_map(|col| char::from_u32(0x4E00 + row * per_row + col))
            .collect();
        let size = ((row % 5) as f32).mul_add(3.0, 8.0);
        let style = TextStyle::default()
            .font_family("system-ui")
            .font_size(size);
        let area = rect(0.0, (row % 64) as f32 * 8.0, 1024.0, size * 1.5);
        commands.push(SceneCommand::GlyphRun(shape_glyph_run(
            area, text, INK, style, 10_000,
        )));
    }
    scene(commands, (1024.0, 512.0), 1.0)
}

/// Builds a 512 by 512 scene of `count` images, each `side` pixels square in a solid color taken from its index and drawn 8 by 8.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn unique_images(count: u32, side: u32) -> GeneratedScene {
    let mut commands = Vec::new();
    for i in 0..count {
        let bytes = i.to_le_bytes();
        let rgba: Vec<u8> = (0..side * side)
            .flat_map(|_| [bytes[0], bytes[1], bytes[2], 255])
            .collect();
        let Some(source) = ImageSource::from_rgba8(side, side, rgba) else {
            continue;
        };
        commands.push(SceneCommand::Image(DrawImage {
            node: None,
            source,
            fit: ObjectFit::Fill,
            destination: rect(
                (i % 64) as f32 * 8.0,
                ((i / 64) % 64) as f32 * 8.0,
                8.0,
                8.0,
            ),
            opacity: 10_000,
            crop: None,
            raster: None,
        }));
    }
    scene(commands, (512.0, 512.0), 1.0)
}

/// Builds a scene of one rectangle that fills a `width` by `height` viewport at device scale `scale`.
#[must_use]
pub fn viewport(width: f32, height: f32, scale: f32) -> GeneratedScene {
    scene(
        vec![fill(rect(0.0, 0.0, width, height))],
        (width, height),
        scale,
    )
}
