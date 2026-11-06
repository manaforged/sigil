use layout::backends::svg::SVGWriter;
use layout::core::format::{ClipHandle, RenderBackend};
use layout::core::geometry::Point;
use layout::core::style::StyleAttr;

const MARGIN: f64 = 6.0;
const GLYPH_WIDTH: f64 = 0.6;

pub(super) struct Tight {
    writer: SVGWriter,
    min: (f64, f64),
    max: (f64, f64),
}

impl Tight {
    pub(super) fn new() -> Self {
        Self {
            writer: SVGWriter::new(),
            min: (f64::INFINITY, f64::INFINITY),
            max: (f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    fn cover(&mut self, (x, y): (f64, f64), (half_w, half_h): (f64, f64)) {
        self.min = (self.min.0.min(x - half_w), self.min.1.min(y - half_h));
        self.max = (self.max.0.max(x + half_w), self.max.1.max(y + half_h));
    }

    pub(super) fn finish(&self) -> Option<String> {
        let svg = self.writer.finalize();
        let (x, y) = (self.min.0 - MARGIN, self.min.1 - MARGIN);
        let (w, h) = (self.max.0 - x + MARGIN, self.max.1 - y + MARGIN);
        if !(w.is_finite() && h.is_finite()) {
            return None;
        }
        let start = svg.find("<svg ")?;
        let end = start + svg.get(start..)?.find("xmlns")?;
        let header = format!("<svg width=\"{w}\" height=\"{h}\" viewBox=\"{x} {y} {w} {h}\" ");
        Some(format!("{}{header}{}", svg.get(..start)?, svg.get(end..)?))
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "label lengths and font sizes are far below 2^52"
)]
fn text_extent(text: &str, font_size: usize) -> (f64, f64) {
    let longest = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let lines = text.lines().count().max(1);
    let size = font_size as f64;
    (
        longest as f64 * size * GLYPH_WIDTH / 2.0,
        lines as f64 * size / 2.0,
    )
}

impl RenderBackend for Tight {
    fn draw_rect(
        &mut self,
        xy: Point,
        size: Point,
        look: &StyleAttr,
        properties: Option<String>,
        clip: Option<ClipHandle>,
    ) {
        self.cover(
            (xy.x + size.x / 2.0, xy.y + size.y / 2.0),
            (size.x / 2.0, size.y / 2.0),
        );
        self.writer.draw_rect(xy, size, look, properties, clip);
    }

    fn draw_line(
        &mut self,
        start: Point,
        stop: Point,
        look: &StyleAttr,
        properties: Option<String>,
    ) {
        self.cover((start.x, start.y), (0.0, 0.0));
        self.cover((stop.x, stop.y), (0.0, 0.0));
        self.writer.draw_line(start, stop, look, properties);
    }

    fn draw_circle(
        &mut self,
        xy: Point,
        size: Point,
        look: &StyleAttr,
        properties: Option<String>,
    ) {
        self.cover((xy.x, xy.y), (size.x / 2.0, size.y / 2.0));
        self.writer.draw_circle(xy, size, look, properties);
    }

    fn draw_text(&mut self, xy: Point, text: &str, look: &StyleAttr) {
        self.cover((xy.x, xy.y), text_extent(text, look.font_size));
        self.writer.draw_text(xy, text, look);
    }

    fn draw_arrow(
        &mut self,
        path: &[(Point, Point)],
        dashed: bool,
        head: (bool, bool),
        look: &StyleAttr,
        properties: Option<String>,
        text: &str,
    ) {
        let pad = if text.is_empty() {
            0.0
        } else {
            text_extent(text, look.font_size).1 * 2.0
        };
        for (from, to) in path {
            self.cover((from.x, from.y), (pad, pad));
            self.cover((to.x, to.y), (pad, pad));
        }
        self.writer
            .draw_arrow(path, dashed, head, look, properties, text);
    }

    fn create_clip(&mut self, xy: Point, size: Point, rounded_px: usize) -> ClipHandle {
        self.writer.create_clip(xy, size, rounded_px)
    }
}
