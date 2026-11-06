use crate::layout::{LayoutStyle, Position};
use crate::style::{Token, Tone};
use crate::widgets::Widget;

use super::super::vector::{PathNode, path};
use super::decimate::{self, Candle};
use super::span::ValueSpan;

pub(super) const VIEW_W: u32 = 400;
pub(super) const VIEW_H: u32 = 200;
pub(super) const VIEW_W_F: f32 = 400.0;
pub(super) const VIEW_H_F: f32 = 200.0;
pub(super) const INSET: f32 = 4.0;
const LINE_STROKE: f32 = 2.0;
const WICK_STROKE: f32 = 1.0;
const BODY_SHARE: f32 = 0.6;
const MIN_BODY: f32 = 1.0;
const RISING: Tone = Tone::Success;
const FALLING: Tone = Tone::Error;

pub(super) fn layer_layout() -> LayoutStyle {
    LayoutStyle::block()
        .position(Position::Absolute)
        .inset_top(0.0)
        .inset_left(0.0)
        .fill()
}

pub(super) fn y_of(span: ValueSpan, value: f32) -> f32 {
    (1.0 - span.fraction(value)).mul_add(2.0f32.mul_add(-INSET, VIEW_H_F), INSET)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "point counts stay far below f32 precision limits at render scale"
)]
fn x_of(index: usize, count: usize) -> f32 {
    VIEW_W_F * (index as f32 + 0.5) / count.max(1) as f32
}

pub(super) struct Trace<'a> {
    pub(super) values: &'a [f32],
    pub(super) count: usize,
    pub(super) span: ValueSpan,
    pub(super) columns: usize,
}

impl Trace<'_> {
    fn points(&self) -> Vec<(f32, f32)> {
        let shown = self
            .values
            .get(..self.count.min(self.values.len()))
            .unwrap_or(&[]);
        decimate::min_max(shown, self.columns)
            .into_iter()
            .map(|(index, value)| (x_of(index, self.count), value))
            .collect()
    }
}

pub(super) fn line(trace: &Trace<'_>, color: Token) -> PathNode {
    let mut stroke = path(VIEW_W, VIEW_H);
    let mut pen_down = false;
    for (x, value) in trace.points() {
        if !value.is_finite() {
            pen_down = false;
            continue;
        }
        let y = y_of(trace.span, value);
        stroke = if pen_down {
            stroke.line_to(x, y)
        } else {
            stroke.move_to(x, y)
        };
        pen_down = true;
    }
    stroke
        .stroke(color, LINE_STROKE)
        .non_scaling_stroke()
        .layout(layer_layout())
}

pub(super) fn area(trace: &Trace<'_>, color: Token) -> PathNode {
    let points = trace
        .points()
        .into_iter()
        .filter(|(_, value)| value.is_finite())
        .collect::<Vec<_>>();
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return path(VIEW_W, VIEW_H).layout(layer_layout());
    };
    let base = y_of(trace.span, 0.0_f32.clamp(trace.span.lo, trace.span.hi));
    let shape = points.iter().fold(
        path(VIEW_W, VIEW_H).move_to(first.0, base),
        |shape, (x, value)| shape.line_to(*x, y_of(trace.span, *value)),
    );
    shape
        .line_to(last.0, base)
        .close()
        .fill(color)
        .layout(layer_layout())
}

#[allow(
    clippy::cast_precision_loss,
    reason = "candle counts stay far below f32 precision limits at render scale"
)]
pub(super) fn candles(rows: &[Candle], span: ValueSpan, columns: usize) -> [PathNode; 2] {
    let rows = decimate::candles(rows, columns);
    let count = rows.len();
    let half = VIEW_W_F / count.max(1) as f32 * BODY_SHARE / 2.0;
    let empty = (path(VIEW_W, VIEW_H), path(VIEW_W, VIEW_H));
    let (up, down) = rows
        .iter()
        .enumerate()
        .fold(empty, |(up, down), (index, row)| {
            let x = x_of(index, count);
            if row.close >= row.open {
                (candle(up, x, half, *row, span), down)
            } else {
                (up, candle(down, x, half, *row, span))
            }
        });
    [finish(up, RISING), finish(down, FALLING)]
}

fn candle(shape: PathNode, x: f32, half: f32, row: Candle, span: ValueSpan) -> PathNode {
    let top = y_of(span, row.open.max(row.close));
    let bottom = y_of(span, row.open.min(row.close)).max(top + MIN_BODY);
    shape
        .move_to(x, y_of(span, row.high))
        .line_to(x, y_of(span, row.low))
        .move_to(x - half, top)
        .line_to(x + half, top)
        .line_to(x + half, bottom)
        .line_to(x - half, bottom)
        .close()
}

fn finish(shape: PathNode, tone: Tone) -> PathNode {
    shape
        .fill(tone.fill())
        .stroke(tone.fill(), WICK_STROKE)
        .non_scaling_stroke()
        .layout(layer_layout())
}
