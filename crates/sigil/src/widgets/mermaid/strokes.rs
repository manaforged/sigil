use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle, Position};
use crate::style::{BORDER_WIDTH, Token, space};
use crate::widgets::vector::{PathNode, path};

use super::canvas::{DOWN, LEFT, RIGHT, UP, bits};

const CELL_W: f32 = space(2);
const CELL_H: f32 = space(4);
const CIRCLE_K: f32 = 0.552_284_8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Line(u8),
    Round(u8),
    Dash(u8),
    Arrow(u8),
    Dot,
}

const fn mark(ch: char) -> Option<Mark> {
    match ch {
        '┄' => Some(Mark::Dash(LEFT | RIGHT)),
        '┆' => Some(Mark::Dash(UP | DOWN)),
        '╭' => Some(Mark::Round(DOWN | RIGHT)),
        '╮' => Some(Mark::Round(DOWN | LEFT)),
        '╰' => Some(Mark::Round(UP | RIGHT)),
        '╯' => Some(Mark::Round(UP | LEFT)),
        '▼' => Some(Mark::Arrow(DOWN)),
        '▲' => Some(Mark::Arrow(UP)),
        '▶' => Some(Mark::Arrow(RIGHT)),
        '◀' => Some(Mark::Arrow(LEFT)),
        '●' => Some(Mark::Dot),
        _ => match bits(ch) {
            Some(bits) => Some(Mark::Line(bits)),
            None => None,
        },
    }
}

fn at(index: usize, cell: f32) -> f32 {
    f32::from(u16::try_from(index).unwrap_or(u16::MAX)) * cell
}

fn stroke_width() -> f32 {
    f32::from(BORDER_WIDTH.0)
}

pub(super) fn window(root: NodeBuilder, lines: &[String]) -> NodeBuilder {
    let grid: Vec<Vec<char>> = lines.iter().map(|line| line.chars().collect()).collect();
    let cols = grid.iter().map(Vec::len).max().unwrap_or(0);
    let size = (at(cols, CELL_W), at(grid.len(), CELL_H));
    let mut body = container().layout(
        LayoutStyle::flex_column()
            .fixed_size(size.0, size.1)
            .shrink(0.0),
    );
    body = body
        .child(layer(strokes(&grid, size), size))
        .child(layer(fills(&grid, size), size));
    for row in &grid {
        body = body.child(text_row(row));
    }
    root.child(body)
}

fn layer(node: PathNode, size: (f32, f32)) -> PathNode {
    node.retina().layout(
        LayoutStyle::block()
            .position(Position::Absolute)
            .inset_top(0.0)
            .inset_left(0.0)
            .fixed_size(size.0, size.1),
    )
}

const fn canvas(size: (f32, f32)) -> PathNode {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "grid extents are bounded by the mermaid output limits"
    )]
    let (width, height) = (size.0.ceil() as u32, size.1.ceil() as u32);
    path(width, height)
}

fn text_row(row: &[char]) -> NodeBuilder {
    let mut node = container().layout(
        LayoutStyle::flex_row()
            .height(CELL_H)
            .shrink(0.0)
            .align_items(AlignItems::Center),
    );
    for run in row.chunk_by(|a, b| is_word(*a) == is_word(*b)) {
        let width = at(run.len(), CELL_W);
        node = node.child(if run.first().copied().is_some_and(is_word) {
            word_cell(run.iter().collect::<String>(), width)
        } else {
            container().layout(LayoutStyle::block().width(width).shrink(0.0))
        });
    }
    node
}

const fn is_word(ch: char) -> bool {
    !ch.is_whitespace() && mark(ch).is_none()
}

fn word_cell(word: String, width: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .width(width)
                .shrink(0.0)
                .justify_content(JustifyContent::Center),
        )
        .child(text(word).nowrap())
}

fn cells(grid: &[Vec<char>]) -> impl Iterator<Item = (f32, f32, Mark)> + '_ {
    grid.iter().enumerate().flat_map(|(y, row)| {
        row.iter()
            .enumerate()
            .filter_map(move |(x, ch)| mark(*ch).map(|found| (at(x, CELL_W), at(y, CELL_H), found)))
    })
}

fn strokes(grid: &[Vec<char>], size: (f32, f32)) -> PathNode {
    let mut node = canvas(size);
    for (y, row) in grid.iter().enumerate() {
        let spans = row.iter().map(|ch| mark(*ch).and_then(h_span));
        let cy = at(y, CELL_H) + CELL_H / 2.0;
        node = runs(node, spans, CELL_W, |from, to| ((from, cy), (to, cy)));
    }
    for x in 0..grid.iter().map(Vec::len).max().unwrap_or(0) {
        let spans = grid
            .iter()
            .map(|row| row.get(x).and_then(|ch| mark(*ch)).and_then(v_span));
        let cx = at(x, CELL_W) + CELL_W / 2.0;
        node = runs(node, spans, CELL_H, |from, to| ((cx, from), (cx, to)));
    }
    for (x0, y0, found) in cells(grid) {
        node = detail(node, x0, y0, found);
    }
    node.stroke(Token::OnSurfaceMuted, stroke_width())
}

type Segment = ((f32, f32), (f32, f32));

fn runs(
    mut node: PathNode,
    spans: impl Iterator<Item = Option<(f32, f32)>>,
    cell: f32,
    place: impl Fn(f32, f32) -> Segment,
) -> PathNode {
    let mut merged: Vec<(f32, f32)> = Vec::new();
    for (index, span) in spans.enumerate() {
        let Some((from, to)) = span else { continue };
        let (start, end) = (at(index, cell) + from, at(index, cell) + to);
        match merged.last_mut() {
            Some(last) if (last.1 - start).abs() < f32::EPSILON => last.1 = end,
            _ => merged.push((start, end)),
        }
    }
    for (from, to) in merged {
        let (a, b) = place(from, to);
        node = node.move_to(a.0, a.1).line_to(b.0, b.1);
    }
    node
}

fn span(bits: u8, low: u8, high: u8, cell: f32) -> Option<(f32, f32)> {
    if bits & (low | high) == 0 {
        return None;
    }
    let half = stroke_width() / 2.0;
    let from = if bits & low == 0 {
        cell / 2.0 - half
    } else {
        0.0
    };
    let to = if bits & high == 0 {
        cell / 2.0 + half
    } else {
        cell
    };
    Some((from, to))
}

fn h_span(found: Mark) -> Option<(f32, f32)> {
    match found {
        Mark::Line(bits) => span(bits, LEFT, RIGHT, CELL_W),
        _ => None,
    }
}

fn v_span(found: Mark) -> Option<(f32, f32)> {
    match found {
        Mark::Line(bits) => span(bits, UP, DOWN, CELL_H),
        Mark::Arrow(DOWN) => Some((0.0, CELL_H - CELL_W)),
        Mark::Arrow(UP) => Some((CELL_W, CELL_H)),
        _ => None,
    }
}

fn detail(node: PathNode, x0: f32, y0: f32, found: Mark) -> PathNode {
    let (cx, cy) = (x0 + CELL_W / 2.0, y0 + CELL_H / 2.0);
    match found {
        Mark::Round(bits) => {
            let side_x = if bits & RIGHT == 0 { x0 } else { x0 + CELL_W };
            let side_y = if bits & DOWN == 0 { y0 } else { y0 + CELL_H };
            node.move_to(side_x, cy).quad_to(cx, cy, cx, side_y)
        }
        Mark::Dash(bits) if bits & LEFT != 0 => node
            .move_to(x0 + CELL_W / 4.0, cy)
            .line_to(CELL_W.mul_add(0.75, x0), cy),
        Mark::Dash(_) => node
            .move_to(cx, y0 + CELL_H / 4.0)
            .line_to(cx, CELL_H.mul_add(0.75, y0)),
        _ => node,
    }
}

fn fills(grid: &[Vec<char>], size: (f32, f32)) -> PathNode {
    let mut node = canvas(size);
    for (x0, y0, found) in cells(grid) {
        node = match found {
            Mark::Arrow(dir) => arrow(node, x0, y0, dir),
            Mark::Dot => dot(node, x0 + CELL_W / 2.0, y0 + CELL_H / 2.0, CELL_W / 2.0),
            _ => node,
        };
    }
    node.fill(Token::OnSurfaceMuted)
}

fn arrow(node: PathNode, x0: f32, y0: f32, dir: u8) -> PathNode {
    let (cx, cy, half) = (x0 + CELL_W / 2.0, y0 + CELL_H / 2.0, CELL_W / 2.0);
    let (tip, left, right) = match dir {
        DOWN => (
            (cx, y0 + CELL_H),
            (cx - half, y0 + CELL_H - CELL_W),
            (cx + half, y0 + CELL_H - CELL_W),
        ),
        UP => ((cx, y0), (cx - half, y0 + CELL_W), (cx + half, y0 + CELL_W)),
        RIGHT => ((x0 + CELL_W, cy), (x0, cy - half), (x0, cy + half)),
        _ => ((x0, cy), (x0 + CELL_W, cy - half), (x0 + CELL_W, cy + half)),
    };
    node.move_to(tip.0, tip.1)
        .line_to(left.0, left.1)
        .line_to(right.0, right.1)
        .close()
}

fn dot(node: PathNode, cx: f32, cy: f32, radius: f32) -> PathNode {
    let k = radius * CIRCLE_K;
    node.move_to(cx + radius, cy)
        .cubic_to(cx + radius, cy + k, cx + k, cy + radius, cx, cy + radius)
        .cubic_to(cx - k, cy + radius, cx - radius, cy + k, cx - radius, cy)
        .cubic_to(cx - radius, cy - k, cx - k, cy - radius, cx, cy - radius)
        .cubic_to(cx + k, cy - radius, cx + radius, cy - k, cx + radius, cy)
        .close()
}
