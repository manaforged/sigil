use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{LayoutStyle, Position};
use crate::style::{BORDER_WIDTH, Token, Tone};
use crate::widgets::Widget;
use crate::widgets::vector::PathBuilder;

use super::super::flow_vector::{
    Area, CELL_H, CELL_W, HEAD, Point, barb, canvas, cells, dashed_line, edge_label, edge_width,
    layer, node_label, polyline, rounded,
};
use super::super::label::width;
use super::{Diagram, SequenceEvent, centres, depth, height};

const LOOP: f32 = 28.0;
const BOX_INSET: f32 = 4.0;
const RADIUS: f32 = 4.0;

struct Pens {
    lines: PathBuilder,
    dashes: PathBuilder,
    heads: PathBuilder,
    open_heads: PathBuilder,
    boxes: PathBuilder,
    notes: PathBuilder,
    frames: PathBuilder,
    frame_dashes: PathBuilder,
    labels: Vec<NodeBuilder>,
}

impl Pens {
    fn new(size: (f32, f32)) -> Self {
        Self {
            lines: canvas(size),
            dashes: canvas(size),
            heads: canvas(size),
            open_heads: canvas(size),
            boxes: canvas(size),
            notes: canvas(size),
            frames: canvas(size),
            frame_dashes: canvas(size),
            labels: Vec::new(),
        }
    }

    fn compose(self, size: (f32, f32)) -> NodeBuilder {
        let thin = f32::from(BORDER_WIDTH);
        let mut root = container()
            .layout(
                LayoutStyle::block()
                    .position(Position::Relative)
                    .fixed_size(size.0, size.1)
                    .shrink(0.0),
            )
            .child(layer(self.frames.stroke(Token::Border, thin), size))
            .child(layer(self.frame_dashes.stroke(Token::Border, thin), size))
            .child(layer(
                self.lines.stroke(Token::OnSurfaceMuted, edge_width()),
                size,
            ))
            .child(layer(
                self.dashes.stroke(Token::OnSurfaceMuted, edge_width()),
                size,
            ))
            .child(layer(self.heads.fill(Token::OnSurfaceMuted), size))
            .child(layer(
                self.open_heads.stroke(Token::OnSurfaceMuted, edge_width()),
                size,
            ))
            .child(layer(
                self.boxes.fill(Token::Surface).stroke(Token::Border, thin),
                size,
            ))
            .child(layer(
                self.notes
                    .fill(Tone::Primary.step(3))
                    .stroke(Tone::Primary.step(7), thin),
                size,
            ));
        for label in self.labels {
            root = root.child(label);
        }
        root
    }
}

fn x_at(col: usize) -> f32 {
    cells(col, CELL_W) + CELL_W / 2.0
}

fn y_at(row: usize) -> f32 {
    cells(row, CELL_H) + CELL_H / 2.0
}

/// A sequence diagram drawn as vector shapes, with its width.
pub(in super::super) fn figure(diagram: &Diagram) -> (NodeBuilder, f32) {
    let depth = depth(&diagram.events);
    let margin = depth.saturating_mul(2).saturating_add(1);
    let (centres, right) = centres(diagram, margin);
    let rows = diagram.events.iter().fold(7_usize, |rows, event| {
        let same = matches!(event, SequenceEvent::Message { from, to, .. } if from == to);
        rows.saturating_add(height(event, same))
    });
    let cols = right.saturating_add(margin);
    let size = (
        note_extent(diagram, &centres).fold(cells(cols, CELL_W), f32::max),
        cells(rows, CELL_H),
    );
    let mut pens = Pens::new(size);
    lifelines(&mut pens, diagram, &centres, rows);
    events(&mut pens, diagram, &centres, cols);
    (pens.compose(size), size.0)
}

fn note_extent<'a>(diagram: &'a Diagram, centres: &'a [usize]) -> impl Iterator<Item = f32> + 'a {
    diagram.events.iter().filter_map(|event| match event {
        SequenceEvent::Note { from, to, text } if from == to => centres.get(*from).map(|centre| {
            2.0f32.mul_add(CELL_W, x_at(*centre)) + cells(width(text).saturating_add(5), CELL_W)
        }),
        _ => None,
    })
}

fn lifelines(pens: &mut Pens, diagram: &Diagram, centres: &[usize], rows: usize) {
    for centre in centres {
        let x = x_at(*centre);
        with(&mut pens.frame_dashes, |pen| {
            dashed_line(
                pen,
                &[
                    (x, cells(3, CELL_H)),
                    (x, cells(rows.saturating_sub(3), CELL_H)),
                ],
            )
        });
    }
    for (participant, centre) in diagram.participants.iter().zip(centres) {
        for top in [0, rows.saturating_sub(3)] {
            let area = participant_area(&participant.label, *centre, top);
            with(&mut pens.boxes, |pen| rounded(pen, area, RADIUS));
            pens.labels
                .push(node_label(std::slice::from_ref(&participant.label), area));
        }
    }
}

fn participant_area(label: &str, centre: usize, top: usize) -> Area {
    let w = cells(label.chars().count().saturating_add(4), CELL_W);
    Area {
        x: x_at(centre) - w / 2.0,
        y: cells(top, CELL_H) + BOX_INSET,
        w,
        h: (-2.0f32).mul_add(BOX_INSET, cells(3, CELL_H)),
    }
}

fn events(pens: &mut Pens, diagram: &Diagram, centres: &[usize], cols: usize) {
    let mut y = 3_usize;
    let mut open: Vec<(usize, String, usize)> = Vec::new();
    let mut number = 0_usize;
    for event in &diagram.events {
        let at = |index: usize| centres.get(index).copied().unwrap_or(0);
        y = match event {
            SequenceEvent::Message {
                from,
                to,
                text,
                dashed,
                head,
            } => {
                number = number.saturating_add(1);
                let mut text = text.clone();
                if diagram.numbered
                    && let Some(first) = text.first_mut()
                {
                    *first = format!("{number}. {first}");
                }
                message(pens, (at(*from), at(*to)), &text, (*dashed, *head), y)
            }
            SequenceEvent::Note { from, to, text } => note(pens, (at(*from), at(*to)), text, y),
            SequenceEvent::Open { kind, text } => {
                open.push((y, format!("{kind} {text}").trim().to_string(), open.len()));
                y.saturating_add(1)
            }
            SequenceEvent::Else { text } => {
                if let Some((_, _, level)) = open.last() {
                    divider(pens, *level, cols, y, text);
                }
                y.saturating_add(1)
            }
            SequenceEvent::Close => {
                if let Some((top, label, level)) = open.pop() {
                    frame(pens, (level, cols), (top, y), &label);
                }
                y.saturating_add(1)
            }
        };
    }
    for (top, label, level) in open {
        frame(pens, (level, cols), (top, y), &label);
    }
}

fn message(
    pens: &mut Pens,
    ends: (usize, usize),
    text: &[String],
    style: (bool, char),
    y: usize,
) -> usize {
    let (dashed, head) = style;
    let arrow = y.saturating_add(text.len());
    let (a, b) = (x_at(ends.0), x_at(ends.1));
    let line_y = y_at(arrow);
    if ends.0 == ends.1 {
        for (row, part) in text.iter().enumerate() {
            pens.labels.push(caption(
                part,
                a + LOOP + CELL_W,
                y_at(y.saturating_add(row)),
            ));
        }
        let back = line_y + CELL_H;
        let points = [
            (a, line_y),
            (a + LOOP, line_y),
            (a + LOOP, back),
            (a + HEAD, back),
        ];
        stroke(pens, &points, dashed);
        with(&mut pens.heads, |pen| {
            filled_head(pen, (a + HEAD, back), (a, back))
        });
        return arrow.saturating_add(2);
    }
    let middle = f32::midpoint(a, b);
    for (row, part) in text.iter().enumerate() {
        pens.labels
            .push(centred_caption(part, middle, y_at(y.saturating_add(row))));
    }
    let direction = (b - a).signum();
    let base = (-direction).mul_add(HEAD, b);
    stroke(pens, &[(a, line_y), (base, line_y)], dashed);
    let tip = (b, line_y);
    match head {
        '▶' => with(&mut pens.heads, |pen| filled_head(pen, (base, line_y), tip)),
        '✕' => with(&mut pens.open_heads, |pen| cross(pen, tip, direction)),
        _ => with(&mut pens.open_heads, |pen| open_head(pen, tip, direction)),
    }
    arrow.saturating_add(1)
}

fn stroke(pens: &mut Pens, points: &[Point], dashed: bool) {
    if dashed {
        with(&mut pens.dashes, |pen| dashed_line(pen, points));
    } else {
        with(&mut pens.lines, |pen| polyline(pen, points));
    }
}

fn filled_head(node: PathBuilder, base: Point, tip: Point) -> PathBuilder {
    let (dx, dy) = (tip.0 - base.0, tip.1 - base.1);
    let length = dx.hypot(dy).max(f32::EPSILON);
    let (ux, uy) = (dx / length, dy / length);
    barb(node, tip, base, (ux, uy), HEAD * 0.55)
}

fn open_head(node: PathBuilder, tip: Point, direction: f32) -> PathBuilder {
    let back = (-direction).mul_add(HEAD, tip.0);
    node.move_to(back, HEAD.mul_add(-0.6, tip.1))
        .line_to(tip.0, tip.1)
        .line_to(back, HEAD.mul_add(0.6, tip.1))
        .move_to(back, tip.1)
        .line_to(tip.0, tip.1)
}

fn cross(node: PathBuilder, tip: Point, direction: f32) -> PathBuilder {
    let (cx, half) = (tip.0 - direction * HEAD / 2.0, HEAD / 2.0);
    node.move_to(cx - half, tip.1 - half)
        .line_to(cx + half, tip.1 + half)
        .move_to(cx - half, tip.1 + half)
        .line_to(cx + half, tip.1 - half)
}

fn note(pens: &mut Pens, span: (usize, usize), text: &[String], y: usize) -> usize {
    let (from, to) = (span.0.min(span.1), span.0.max(span.1));
    let w = cells(width(text).saturating_add(4), CELL_W);
    let x = if from == to {
        2.0f32.mul_add(CELL_W, x_at(from))
    } else {
        f32::midpoint(x_at(from), x_at(to)) - w / 2.0
    };
    let rows = text.len().saturating_add(2);
    let area = Area {
        x,
        y: cells(y, CELL_H) + BOX_INSET,
        w,
        h: (-2.0f32).mul_add(BOX_INSET, cells(rows, CELL_H)),
    };
    with(&mut pens.notes, |pen| rounded(pen, area, RADIUS));
    pens.labels.push(node_label(text, area));
    y.saturating_add(rows)
}

fn frame(pens: &mut Pens, span: (usize, usize), rows: (usize, usize), label: &str) {
    let (level, cols) = span;
    let (left, right) = frame_edges(level, cols);
    let (top, bottom) = (y_at(rows.0), y_at(rows.1));
    if right - left <= 2.0 * CELL_W {
        return;
    }
    let area = Area {
        x: left,
        y: top,
        w: right - left,
        h: bottom - top,
    };
    with(&mut pens.frames, |pen| rounded(pen, area, RADIUS));
    pens.labels.push(tag(label, left + CELL_W, top));
}

fn frame_edges(level: usize, cols: usize) -> (f32, f32) {
    (
        cells(level, CELL_W) + CELL_W / 2.0,
        cells(cols.saturating_sub(level), CELL_W) - CELL_W / 2.0,
    )
}

fn divider(pens: &mut Pens, level: usize, cols: usize, row: usize, label: &str) {
    let (left, right) = frame_edges(level, cols);
    let y = y_at(row);
    with(&mut pens.frame_dashes, |pen| {
        dashed_line(pen, &[(left, y), (right, y)])
    });
    if !label.is_empty() {
        pens.labels.push(tag(label, left + CELL_W, y));
    }
}

fn caption(part: &str, left: f32, centre_y: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .position(Position::Absolute)
                .inset_left(left)
                .inset_top(centre_y - CELL_H / 2.0),
        )
        .child(text(part.to_string()).class("text-sm").nowrap())
}

fn centred_caption(part: &str, centre_x: f32, centre_y: f32) -> NodeBuilder {
    let w = cells(part.chars().count().saturating_add(2), CELL_W);
    container()
        .layout(
            LayoutStyle::flex_row()
                .position(Position::Absolute)
                .inset_left(centre_x - w / 2.0)
                .inset_top(centre_y - CELL_H / 2.0)
                .fixed_size(w, CELL_H)
                .justify_content(crate::layout::JustifyContent::Center),
        )
        .child(text(part.to_string()).class("text-sm").nowrap())
}

fn tag(label: &str, left: f32, centre_y: f32) -> NodeBuilder {
    let half = f32::midpoint(cells(label.chars().count(), CELL_W), 2.0 * CELL_W);
    edge_label(label, (left + half, centre_y))
}

fn with(pen: &mut PathBuilder, draw: impl FnOnce(PathBuilder) -> PathBuilder) {
    let taken = std::mem::replace(pen, canvas((1.0, 1.0)));
    *pen = draw(taken);
}
