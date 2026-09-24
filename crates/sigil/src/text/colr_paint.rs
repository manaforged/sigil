#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use skrifa::color::{Brush, ColorPainter, ColorStop, CompositeMode, Extend, Transform};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlineGlyphCollection, OutlinePen};
use skrifa::raw::TableProvider;
use skrifa::raw::types::BoundingBox;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use tiny_skia::{
    BlendMode, Color, FillRule, GradientStop, LinearGradient, Mask, Paint, Path, PathBuilder,
    Pixmap, PixmapPaint, Point, RadialGradient, Rect, Shader, SpreadMode, SweepGradient,
};

use super::color_glyph::ColorGlyphRaster;
use super::colr_radial::{Radial, Stop, cut_negative};
use crate::hal::Color as TextColor;

const ASCENT_FRAC: f32 = 0.85;
const FOREGROUND: u16 = 0xFFFF;

#[must_use]
pub(super) fn has_colr(font: &FontRef<'_>) -> bool {
    font.colr().is_ok()
}

#[must_use]
pub(super) fn rasterize(
    font: &FontRef<'_>,
    gid: GlyphId,
    px_per_em: f32,
    foreground: TextColor,
) -> Option<ColorGlyphRaster> {
    let glyph = font.color_glyphs().get(gid)?;
    let (side, root) = frame(font, gid, px_per_em)?;
    let mut painter = ColrPainter {
        target: Pixmap::new(side, side)?,
        layers: Vec::new(),
        transforms: vec![root],
        clips: Vec::new(),
        outlines: font.outline_glyphs(),
        palette: palette_colors(font),
        foreground: to_skia(foreground),
    };
    glyph.paint(LocationRef::default(), &mut painter).ok()?;
    Some(ColorGlyphRaster {
        width: side,
        height: side,
        rgba: demultiply(&painter.target),
        ppem: px_per_em.max(1.0),
    })
}

fn frame(font: &FontRef<'_>, gid: GlyphId, px_per_em: f32) -> Option<(u32, tiny_skia::Transform)> {
    let upem = f32::from(font.head().ok()?.units_per_em().max(1));
    let scale = px_per_em / upem;
    let em = Size::new(px_per_em);
    let metrics = font.metrics(em, LocationRef::default());
    let ascent = metrics.ascent.max(px_per_em * ASCENT_FRAC);
    let height = ascent - metrics.descent.min(0.0);
    let advance = font
        .glyph_metrics(em, LocationRef::default())
        .advance_width(gid)
        .unwrap_or(px_per_em);
    let extent = height.max(advance).max(px_per_em);
    let side = extent.ceil().max(1.0) as u32;
    let dx = (extent - advance) / 2.0;
    let dy = (extent - height) / 2.0 + ascent;
    Some((
        side,
        tiny_skia::Transform::from_row(scale, 0.0, 0.0, -scale, dx, dy),
    ))
}

fn palette_colors(font: &FontRef<'_>) -> Vec<Color> {
    font.color_palettes()
        .get(0)
        .map(|palette| {
            palette
                .colors()
                .iter()
                .map(|c| Color::from_rgba8(c.red, c.green, c.blue, c.alpha))
                .collect()
        })
        .unwrap_or_default()
}

fn to_skia(color: TextColor) -> Color {
    Color::from_rgba(
        color.r.clamp(0.0, 1.0),
        color.g.clamp(0.0, 1.0),
        color.b.clamp(0.0, 1.0),
        color.a.clamp(0.0, 1.0),
    )
    .unwrap_or(Color::BLACK)
}

fn demultiply(pixmap: &Pixmap) -> Vec<u8> {
    pixmap
        .pixels()
        .iter()
        .flat_map(|px| {
            let c = px.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

struct ColrPainter<'a> {
    target: Pixmap,
    layers: Vec<Pixmap>,
    transforms: Vec<tiny_skia::Transform>,
    clips: Vec<Option<Mask>>,
    outlines: OutlineGlyphCollection<'a>,
    palette: Vec<Color>,
    foreground: Color,
}

impl ColrPainter<'_> {
    fn current(&self) -> tiny_skia::Transform {
        self.transforms.last().copied().unwrap_or_default()
    }

    fn current_clip(&self) -> Option<&Mask> {
        self.clips.last().and_then(Option::as_ref)
    }

    fn clip_mask(&self, path: &Path) -> Option<Mask> {
        let transform = self.current();
        if let Some(existing) = self.current_clip() {
            let mut mask = existing.clone();
            mask.intersect_path(path, FillRule::Winding, true, transform);
            return Some(mask);
        }
        let mut mask = Mask::new(self.target.width(), self.target.height())?;
        mask.fill_path(path, FillRule::Winding, true, transform);
        Some(mask)
    }

    fn push_clip_path(&mut self, path: Option<Path>) {
        let mask = path
            .and_then(|path| self.clip_mask(&path))
            .or_else(|| Mask::new(self.target.width(), self.target.height()));
        self.clips.push(mask);
    }

    fn color(&self, palette_index: u16, alpha: f32) -> Color {
        let mut color = match palette_index {
            FOREGROUND => self.foreground,
            index => self
                .palette
                .get(usize::from(index))
                .copied()
                .unwrap_or(Color::BLACK),
        };
        color.apply_opacity(alpha);
        color
    }

    fn stops(&self, stops: &[ColorStop]) -> Vec<Stop> {
        let mut out: Vec<Stop> = stops
            .iter()
            .map(|s| (s.offset, self.color(s.palette_index, s.alpha)))
            .collect();
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out
    }

    fn shader(&self, brush: &Brush<'_>) -> Option<Shader<'static>> {
        let ts = self.current();
        match *brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => Some(Shader::SolidColor(self.color(palette_index, alpha))),
            Brush::LinearGradient {
                p0,
                p1,
                color_stops,
                extend,
            } => LinearGradient::new(
                Point::from_xy(p0.x, p0.y),
                Point::from_xy(p1.x, p1.y),
                gradient(self.stops(color_stops)),
                spread(extend),
                ts,
            ),
            Brush::RadialGradient {
                c0,
                r0,
                c1,
                r1,
                color_stops,
                extend,
            } => {
                let radial = cut_negative(Radial {
                    c0: Point::from_xy(c0.x, c0.y),
                    r0,
                    c1: Point::from_xy(c1.x, c1.y),
                    r1,
                    stops: self.stops(color_stops),
                })?;
                radial_shader(radial, extend, ts)
            }
            Brush::SweepGradient {
                c0,
                start_angle,
                end_angle,
                color_stops,
                extend,
            } => SweepGradient::new(
                Point::from_xy(c0.x, c0.y),
                start_angle,
                end_angle,
                gradient(self.stops(color_stops)),
                spread(extend),
                ts,
            ),
        }
    }
}

impl ColorPainter for ColrPainter<'_> {
    fn push_transform(&mut self, transform: Transform) {
        let next = tiny_skia::Transform::from_row(
            transform.xx,
            transform.yx,
            transform.xy,
            transform.yy,
            transform.dx,
            transform.dy,
        );
        self.transforms.push(self.current().pre_concat(next));
    }

    fn pop_transform(&mut self) {
        self.transforms.pop();
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        let mut pen = PathPen(PathBuilder::new());
        let drawn = self.outlines.get(glyph_id).and_then(|outline| {
            let settings = DrawSettings::unhinted(Size::unscaled(), LocationRef::default());
            outline.draw(settings, &mut pen).ok()
        });
        self.push_clip_path(drawn.and_then(|_| pen.0.finish()));
    }

    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>) {
        let rect = Rect::from_ltrb(
            clip_box.x_min,
            clip_box.y_min,
            clip_box.x_max,
            clip_box.y_max,
        );
        self.push_clip_path(rect.map(PathBuilder::from_rect));
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        let Some(shader) = self.shader(&brush) else {
            return;
        };
        let Some(area) = tiny_skia::IntSize::from_wh(self.target.width(), self.target.height())
            .map(|size| size.to_int_rect(0, 0).to_rect())
        else {
            return;
        };
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        };
        let mask = self.clips.last().and_then(Option::as_ref);
        self.target
            .fill_rect(area, &paint, tiny_skia::Transform::identity(), mask);
    }

    fn push_layer(&mut self, _composite_mode: CompositeMode) {
        let Some(layer) = Pixmap::new(self.target.width(), self.target.height()) else {
            return;
        };
        self.layers.push(std::mem::replace(&mut self.target, layer));
    }

    fn pop_layer_with_mode(&mut self, composite_mode: CompositeMode) {
        let Some(parent) = self.layers.pop() else {
            return;
        };
        let layer = std::mem::replace(&mut self.target, parent);
        let paint = PixmapPaint {
            blend_mode: blend(composite_mode),
            ..PixmapPaint::default()
        };
        self.target.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &paint,
            tiny_skia::Transform::identity(),
            None,
        );
    }
}

struct PathPen(PathBuilder);

impl OutlinePen for PathPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.0.close();
    }
}

fn radial_shader(
    radial: Radial,
    extend: Extend,
    ts: tiny_skia::Transform,
) -> Option<Shader<'static>> {
    RadialGradient::new(
        radial.c0,
        radial.r0,
        radial.c1,
        radial.r1,
        gradient(radial.stops),
        spread(extend),
        ts,
    )
}

fn gradient(stops: Vec<Stop>) -> Vec<GradientStop> {
    stops
        .into_iter()
        .map(|(offset, color)| GradientStop::new(offset, color))
        .collect()
}

const fn spread(extend: Extend) -> SpreadMode {
    match extend {
        Extend::Repeat => SpreadMode::Repeat,
        Extend::Reflect => SpreadMode::Reflect,
        _ => SpreadMode::Pad,
    }
}

const fn blend(mode: CompositeMode) -> BlendMode {
    match mode {
        CompositeMode::Clear => BlendMode::Clear,
        CompositeMode::Src => BlendMode::Source,
        CompositeMode::Dest => BlendMode::Destination,
        CompositeMode::DestOver => BlendMode::DestinationOver,
        CompositeMode::SrcIn => BlendMode::SourceIn,
        CompositeMode::DestIn => BlendMode::DestinationIn,
        CompositeMode::SrcOut => BlendMode::SourceOut,
        CompositeMode::DestOut => BlendMode::DestinationOut,
        CompositeMode::SrcAtop => BlendMode::SourceAtop,
        CompositeMode::DestAtop => BlendMode::DestinationAtop,
        CompositeMode::Xor => BlendMode::Xor,
        CompositeMode::Plus => BlendMode::Plus,
        CompositeMode::Screen => BlendMode::Screen,
        CompositeMode::Overlay => BlendMode::Overlay,
        CompositeMode::Darken => BlendMode::Darken,
        CompositeMode::Lighten => BlendMode::Lighten,
        CompositeMode::ColorDodge => BlendMode::ColorDodge,
        CompositeMode::ColorBurn => BlendMode::ColorBurn,
        CompositeMode::HardLight => BlendMode::HardLight,
        CompositeMode::SoftLight => BlendMode::SoftLight,
        CompositeMode::Difference => BlendMode::Difference,
        CompositeMode::Exclusion => BlendMode::Exclusion,
        CompositeMode::Multiply => BlendMode::Multiply,
        CompositeMode::HslHue => BlendMode::Hue,
        CompositeMode::HslSaturation => BlendMode::Saturation,
        CompositeMode::HslColor => BlendMode::Color,
        CompositeMode::HslLuminosity => BlendMode::Luminosity,
        _ => BlendMode::SourceOver,
    }
}
