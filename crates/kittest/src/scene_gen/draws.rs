use sigil::geometry::{Point, Rect};
use sigil::kernel::{ImageCrop, ImageSource, ObjectFit};
use sigil::renderer::BUILTIN_SPRITE_ATLAS_ID;
use sigil::renderer::scenes::BUILTIN_ICON_ATLAS_BASE_ID;
use sigil::scene::{
    DecorationKind, DrawDecoration, DrawGradient, DrawImage, DrawPattern, DrawRect, DrawSprite,
    GradientStop, LinearGradient, PatternKind, SceneCommand, SpriteKind, SpriteSource,
    shape_glyph_run,
};
use sigil::style::{TextLineHeight, TextStyle};

use super::Gen;

const TEXTS: [&str; 6] = [
    "Wg@ ffi",
    "",
    " ",
    "مرحبا بالعالم",
    "漢字かな",
    "a\u{301}\u{200d}🙂x",
];
const FONT_SIZES: [f32; 7] = [0.0, 0.5, 6.0, 13.0, 24.0, 96.0, 700.0];
const PERIODS: [f32; 6] = [0.0, 0.01, 1.0, 6.0, 32.0, 1.0e6];

impl Gen<'_, '_, '_> {
    pub(crate) fn draw(&mut self) {
        let command = match self.entropy.below(9) {
            0 | 1 => self.solid(),
            2 => self.gradient(),
            3 => self.pattern(),
            4 => self.shadow_command(),
            5 => self.glyph_run(),
            6 => self.sprite(),
            7 => self.image(),
            _ => self.decoration(),
        };
        self.out.push(command);
    }

    fn solid(&mut self) -> SceneCommand {
        let rect = self.rect();
        let color = self.color();
        let mut draw = DrawRect::new(rect, color)
            .border_radius(self.radii())
            .opacity(self.opacity_u16());
        draw.stroke = self.stroke();
        SceneCommand::Rect(draw)
    }

    fn gradient(&mut self) -> SceneCommand {
        let rect = self.rect();
        let (w, h) = self.viewport;
        let start = Point::new(self.coord(w), self.coord(h));
        let end = if self.entropy.chance(15) {
            start
        } else {
            Point::new(self.coord(w), self.coord(h))
        };
        let count = self.entropy.below(6);
        let mut stops = Vec::new();
        for _ in 0..count {
            let offset = self.entropy.pick(&[0.0, 1.0, 0.5, -0.5, 1.5, 0.25, 0.75]);
            stops.push(GradientStop::new(offset, self.color()));
        }
        let gradient = LinearGradient::new(start, end, stops);
        let mut draw = DrawGradient::new(rect, gradient)
            .border_radius(self.radii())
            .opacity(self.opacity_u16());
        draw.stroke = self.stroke();
        SceneCommand::Gradient(draw)
    }

    fn pattern(&mut self) -> SceneCommand {
        let rect = self.rect();
        let pattern = if self.entropy.chance(50) {
            PatternKind::Slash {
                color: self.color(),
                period: self.entropy.pick(&PERIODS),
                line_width: self.entropy.pick(&PERIODS),
            }
        } else {
            PatternKind::Checker {
                color_a: self.color(),
                color_b: self.color(),
                cell: self.entropy.pick(&PERIODS),
            }
        };
        let mut draw = DrawPattern::new(rect, pattern)
            .border_radius(self.radii())
            .opacity(self.opacity_u16());
        draw.stroke = self.stroke();
        SceneCommand::Pattern(draw)
    }

    fn shadow_command(&mut self) -> SceneCommand {
        let rect = self.rect();
        SceneCommand::DrawShadow {
            rect: [rect.origin.x, rect.origin.y, rect.width, rect.height],
            radii: self.radii(),
            shadow: self.shadow(),
        }
    }

    fn glyph_run(&mut self) -> SceneCommand {
        let rect = self.rect();
        let color = self.color();
        let opacity = self.opacity_u16();
        let e = &mut *self.entropy;
        let text = e.pick(&TEXTS).to_owned();
        let style = TextStyle {
            font_family: String::from(e.pick(&["system-ui", "monospace", "no-such-font"])),
            font_size: e.pick(&FONT_SIZES),
            font_weight: e.pick(&[100, 400, 700, 900]),
            italic: e.chance(20),
            line_height: TextLineHeight::FontSizeRelative(e.pick(&[0.0, 1.2, 3.0])),
            letter_spacing_em: e.pick(&[0.0, -0.5, 0.1, 2.0]),
            ..TextStyle::default()
        };
        SceneCommand::GlyphRun(shape_glyph_run(rect, text, color, style, opacity))
    }

    fn sprite(&mut self) -> SceneCommand {
        let destination = self.rect();
        let color = self.color();
        let opacity = self.opacity_u16();
        let e = &mut *self.entropy;
        let atlas = e.pick(&[
            BUILTIN_SPRITE_ATLAS_ID,
            BUILTIN_ICON_ATLAS_BASE_ID,
            BUILTIN_ICON_ATLAS_BASE_ID + 3,
            7,
        ]);
        let region = Rect::new(
            Point::new(
                e.pick(&[0.0, 200.0, 400.0, 590.0, -10.0]),
                e.pick(&[0.0, 5.0, 190.0]),
            ),
            e.pick(&[0.0, 1.0, 50.0, 128.0, 200.0, 5000.0]),
            e.pick(&[0.0, 1.0, 50.0, 128.0, 200.0, 5000.0]),
        );
        let source = SpriteSource {
            atlas,
            region,
            phase: e.pick(&[0.0, 0.35, 1.0, -2.0]),
        };
        let kind = e.pick(&[SpriteKind::Mono, SpriteKind::Subpixel, SpriteKind::Poly]);
        SceneCommand::Sprite(DrawSprite::new(source, destination, kind, color).opacity(opacity))
    }

    fn image(&mut self) -> SceneCommand {
        let destination = self.rect();
        let opacity = self.opacity_u16();
        let e = &mut *self.entropy;
        let (w, h) = (e.pick(&[1_u32, 2, 7, 33]), e.pick(&[1_u32, 3, 16, 40]));
        let pixels = usize::try_from(w * h * 4).unwrap_or(4);
        let rgba: Vec<u8> = (0..pixels).map(|_| e.byte()).collect();
        let crop = e.chance(25).then(|| ImageCrop {
            top: e.pick(&[0, 1, 5, 100]),
            height: e.pick(&[0, 1, 8, 100]),
        });
        let Some(source) = ImageSource::from_rgba8(w, h, rgba) else {
            return SceneCommand::PopLayer;
        };
        SceneCommand::Image(DrawImage {
            node: None,
            source,
            fit: e.pick(&[ObjectFit::Fill, ObjectFit::Contain, ObjectFit::Cover]),
            destination,
            opacity,
            crop,
            raster: None,
        })
    }

    fn decoration(&mut self) -> SceneCommand {
        let band = self.rect();
        let color = self.color();
        let opacity = self.opacity_u16();
        let e = &mut *self.entropy;
        SceneCommand::Decoration(DrawDecoration {
            node: None,
            kind: e.pick(&[
                DecorationKind::Underline,
                DecorationKind::Strikethrough,
                DecorationKind::WavyUnderline,
            ]),
            band,
            color,
            opacity,
            period: e.pick(&PERIODS),
            line_width: e.pick(&PERIODS),
        })
    }
}
