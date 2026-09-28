use sigil::hal::{Color, Point, Rect};
use sigil::kernel::{ImageSource, ObjectFit};
use sigil::scene::{
    CornerRadii, DrawGlyphRun, DrawImage, DrawRect, LayerEffect, LayerScope, SceneCommand,
    SceneRectStyle, shape_glyph_run,
};
use sigil::style::{Rgba8, TextStyle};

use super::GeneratedScene;

const INK: Rgba8 = Rgba8 {
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
        clear: Color::rgba(0.05, 0.06, 0.09, 1.0),
    }
}

const fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(Point::new(x, y), w, h)
}

const fn fill(area: Rect) -> SceneCommand {
    SceneCommand::Rect(DrawRect::new(area, SceneRectStyle::solid(INK)))
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn clip_depth(depth: usize) -> GeneratedScene {
    let mut commands = Vec::with_capacity(depth * 2 + 1);
    for i in 0..depth {
        let inset = (i as f32) * 0.05;
        commands.push(SceneCommand::PushClip(rect(
            inset,
            inset,
            128.0 - inset,
            128.0 - inset,
        )));
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

#[must_use]
pub fn layer_depth(depth: usize, blur: bool) -> GeneratedScene {
    let mut commands = Vec::with_capacity(depth * 2 + 1);
    for _ in 0..depth {
        let effect = if blur {
            LayerEffect::LayerBlur { sigma: 1.0 }
        } else {
            LayerEffect::Identity
        };
        commands.push(SceneCommand::PushLayer(LayerScope {
            clip: rect(0.0, 0.0, 128.0, 128.0),
            clip_radii: CornerRadii::uniform(4.0),
            effect,
            opacity: 0.99,
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
        let style = TextStyle {
            font_family: String::from("system-ui"),
            font_size: size,
            ..TextStyle::default()
        };
        let area = rect(0.0, (row % 64) as f32 * 8.0, 1024.0, size * 1.5);
        commands.push(SceneCommand::GlyphRun(DrawGlyphRun::new(shape_glyph_run(
            area, text, INK, style, 10_000,
        ))));
    }
    scene(commands, (1024.0, 512.0), 1.0)
}

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
        }));
    }
    scene(commands, (512.0, 512.0), 1.0)
}

#[must_use]
pub fn viewport(width: f32, height: f32, scale: f32) -> GeneratedScene {
    scene(
        vec![fill(rect(0.0, 0.0, width, height))],
        (width, height),
        scale,
    )
}
