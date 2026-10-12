use crate::geometry::{Point, Rect};
use crate::scene::shape_glyph_run;
use crate::scene::{CommandScene, SceneBuilder};
use crate::style::Color;
use crate::style::{TextLineHeight, TextStyle};

use super::helpers::{DARK, rgba};

const LINES: [(&str, f32); 3] = [
    ("The quick brown fox jumps over the lazy dog", 14.0),
    ("Sphinx of black quartz, judge my vow 0123456789", 20.0),
    ("Glyphs at 32 px: Wg@ ffi", 32.0),
];

#[must_use]
pub fn text() -> (CommandScene, (f32, f32), Color) {
    let mut scene = SceneBuilder::new();
    let mut y = 16.0;
    for (line, size) in LINES {
        let style = TextStyle {
            font_family: String::from("system-ui"),
            font_size: size,
            font_weight: 400,
            line_height: TextLineHeight::FontSizeRelative(1.2),
            ..Default::default()
        };
        let rect = Rect::new(Point::new(16.0, y), 728.0, size * 1.5);
        let ink = rgba(236, 240, 245, 255);
        scene.draw_glyph_run(shape_glyph_run(rect, line.to_owned(), ink, style, 10_000));
        y += size * 1.8;
    }
    (scene.finish(), (760.0, 160.0), DARK)
}
