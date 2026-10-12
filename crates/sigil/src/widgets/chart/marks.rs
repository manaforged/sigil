use crate::layout::{LayoutStyle, Position};
use crate::style::Tone;

use super::super::vector::{PathNode, path};
use super::decimate::{self, Candle};
use super::plot::Span;

const VIEW_W: u32 = 400;
const VIEW_H: u32 = 200;
const VIEW_W_F: f32 = 400.0;
const VIEW_H_F: f32 = 200.0;
const INSET: f32 = 4.0;
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

fn y_of(span: Span, value: f32) -> f32 {
    let t = ((value - span.lo) / span.width()).clamp(0.0, 1.0);
    (1.0 - t).mul_add(2.0f32.mul_add(-INSET, VIEW_H_F), INSET)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "point counts stay far below f32 precision limits at render scale"
)]
fn x_of(index: usize, count: usize) -> f32 {
    VIEW_W_F * (index as f32 + 0.5) / count.max(1) as f32
}

pub(super) fn line(
    values: &[f32],
    tone: Tone,
    count: usize,
    span: Span,
    columns: usize,
) -> PathNode {
    let shown = values.get(..count.min(values.len())).unwrap_or(&[]);
    let mut stroke = path(VIEW_W, VIEW_H);
    for (at, (index, value)) in decimate::min_max(shown, columns).into_iter().enumerate() {
        let (x, y) = (x_of(index, count), y_of(span, value));
        stroke = if at == 0 {
            stroke.move_to(x, y)
        } else {
            stroke.line_to(x, y)
        };
    }
    stroke
        .stroke(tone.fill(), LINE_STROKE)
        .non_scaling_stroke()
        .layout(layer_layout())
}

#[allow(
    clippy::cast_precision_loss,
    reason = "candle counts stay far below f32 precision limits at render scale"
)]
pub(super) fn candles(rows: &[Candle], span: Span, columns: usize) -> [PathNode; 2] {
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

fn candle(shape: PathNode, x: f32, half: f32, row: Candle, span: Span) -> PathNode {
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
