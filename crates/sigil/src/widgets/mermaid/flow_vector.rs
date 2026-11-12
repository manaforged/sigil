use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Position};
use crate::scene::geometry::KAPPA as CIRCLE_K;
use crate::style::{BORDER_WIDTH, Paint, Radius, Token, space};
use crate::widgets::Layout;
use crate::widgets::vector::{PathNode, path};

use super::flow::{Graph, GridRect, NodeShape, arrange};
use super::flow_parse::parse;

mod lanes;
mod shapes;

use lanes::route;
use shapes::{dots, shapes};

pub(super) const CELL_W: f32 = 7.0;
pub(super) const CELL_H: f32 = space(4);
const CORNER: f32 = 8.0;
pub(super) const HEAD: f32 = 7.0;
const DASH: f32 = 4.0;
const DASH_GAP: f32 = 3.0;
const DOT: f32 = 6.0;
const OVERLAP: f32 = 12.0;
const LANE: f32 = space(4);
const LABEL_H: f32 = 20.0;
const LABEL_CHAR: f32 = 7.0;

pub(super) type Point = (f32, f32);

#[derive(Clone, Copy)]
pub(super) struct Area {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) w: f32,
    pub(super) h: f32,
}

impl Area {
    fn cx(self) -> f32 {
        self.x + self.w / 2.0
    }

    fn cy(self) -> f32 {
        self.y + self.h / 2.0
    }

    fn right(self) -> f32 {
        self.x + self.w
    }

    fn bottom(self) -> f32 {
        self.y + self.h
    }
}

pub(super) fn cells(count: usize, cell: f32) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX)) * cell
}

fn outer(rect: GridRect) -> Area {
    Area {
        x: cells(rect.x, CELL_W),
        y: cells(rect.y, CELL_H),
        w: cells(rect.w, CELL_W),
        h: cells(rect.h, CELL_H),
    }
}

fn body(rect: GridRect, dummy: bool, vertical: bool) -> Area {
    let area = outer(rect);
    match (dummy, vertical) {
        (true, true) => Area {
            x: area.cx(),
            w: 0.0,
            ..area
        },
        (true, false) => Area {
            y: area.cy(),
            h: 0.0,
            ..area
        },
        (false, _) => Area {
            x: area.x + CELL_W / 2.0,
            y: area.y + CELL_H / 2.0,
            w: area.w - CELL_W,
            h: area.h - CELL_H,
        },
    }
}

/// A flowchart or state diagram drawn as vector shapes, or `None` for other diagrams.
pub(super) fn figure(source: &str) -> Option<(NodeBuilder, f32)> {
    let graph = parse(source)?;
    if graph.nodes.is_empty() {
        return None;
    }
    let (graph, rects) = arrange(graph);
    let vertical = graph.dir.vertical();
    let areas: Vec<Area> = graph
        .nodes
        .iter()
        .zip(&rects)
        .map(|(node, rect)| body(*rect, node.dummy, vertical))
        .collect();
    let size = extent(&rects);
    let margin = if has_back_edge(&graph, &areas, vertical) {
        2.0 * LANE
    } else {
        0.0
    };
    let size = (size.0 + margin, size.1 + margin);
    let mut root = container().layout(
        LayoutStyle::block()
            .position(Position::Relative)
            .fixed_size(size.0, size.1)
            .shrink(0.0),
    );
    let (lines, dashes, heads, labels) = edges(&graph, &areas, size);
    root = root
        .child(layer(
            lines.stroke(Token::OnSurfaceMuted, edge_width()),
            size,
        ))
        .child(layer(
            dashes.stroke(Token::OnSurfaceMuted, edge_width()),
            size,
        ))
        .child(layer(heads.fill(Token::OnSurfaceMuted), size))
        .child(layer(shapes(&graph, &areas, size), size))
        .child(layer(dots(&graph, &areas, size), size));
    for (node, area) in graph.nodes.iter().zip(&areas) {
        if !node.dummy && node.shape != NodeShape::Terminal {
            root = root.child(node_label(&node.label, *area));
        }
    }
    for label in labels {
        root = root.child(label);
    }
    Some((root, size.0))
}

fn extent(rects: &[GridRect]) -> (f32, f32) {
    rects
        .iter()
        .map(|rect| outer(*rect))
        .fold((0.0_f32, 0.0_f32), |size, area| {
            (size.0.max(area.right()), size.1.max(area.bottom()))
        })
}

fn has_back_edge(graph: &Graph, areas: &[Area], vertical: bool) -> bool {
    graph.edges.iter().any(|edge| {
        let (Some(from), Some(to)) = (areas.get(edge.from), areas.get(edge.to)) else {
            return false;
        };
        if vertical {
            to.cy() < from.cy()
        } else {
            to.cx() < from.cx()
        }
    })
}

pub(super) fn edge_width() -> f32 {
    f32::from(BORDER_WIDTH) * 1.25
}

pub(super) fn canvas(size: (f32, f32)) -> PathNode {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "diagram extents are bounded by the mermaid output limits"
    )]
    let (width, height) = (size.0.ceil() as u32, size.1.ceil() as u32);
    path(width.max(1), height.max(1))
}

pub(super) fn layer(node: PathNode, size: (f32, f32)) -> PathNode {
    node.layout(
        LayoutStyle::block()
            .position(Position::Absolute)
            .inset_top(0.0)
            .inset_left(0.0)
            .fixed_size(size.0, size.1),
    )
}

pub(super) fn rounded(node: PathNode, area: Area, radius: f32) -> PathNode {
    let r = radius.min(area.w / 2.0).min(area.h / 2.0);
    let (x0, y0, x1, y1) = (area.x, area.y, area.right(), area.bottom());
    node.move_to(x0 + r, y0)
        .line_to(x1 - r, y0)
        .quad_to(x1, y0, x1, y0 + r)
        .line_to(x1, y1 - r)
        .quad_to(x1, y1, x1 - r, y1)
        .line_to(x0 + r, y1)
        .quad_to(x0, y1, x0, y1 - r)
        .line_to(x0, y0 + r)
        .quad_to(x0, y0, x0 + r, y0)
        .close()
}

pub(super) fn node_label(lines: &[String], area: Area) -> NodeBuilder {
    let mut column = container().layout(
        LayoutStyle::flex_column()
            .position(Position::Absolute)
            .inset_left(area.x)
            .inset_top(area.y)
            .fixed_size(area.w, area.h)
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::Center),
    );
    for line in lines {
        column = column.child(text(line.clone()).class("text-sm").nowrap());
    }
    column
}

type Edges = (PathNode, PathNode, PathNode, Vec<NodeBuilder>);

fn edges(graph: &Graph, areas: &[Area], size: (f32, f32)) -> Edges {
    let vertical = graph.dir.vertical();
    let (mut lines, mut dashes, mut heads) = (canvas(size), canvas(size), canvas(size));
    let mut labels = Vec::new();
    for (index, area) in areas.iter().enumerate() {
        if let Some(node) = graph.nodes.get(index).filter(|node| node.dummy) {
            let through = [(area.x, area.y), (area.right(), area.bottom())];
            if node.dashed {
                dashes = dashed_line(dashes, &through);
            } else {
                lines = polyline(lines, &through);
            }
        }
    }
    for edge in &graph.edges {
        let (Some(from), Some(to)) = (areas.get(edge.from), areas.get(edge.to)) else {
            continue;
        };
        let mut points = route(vertical, *from, *to, areas);
        if edge.arrow {
            heads = arrow_head(heads, &mut points);
        }
        if edge.dashed {
            dashes = dashed_line(dashes, &points);
        } else {
            lines = polyline(lines, &points);
        }
        if !edge.label.is_empty() {
            labels.push(edge_label(&edge.label, middle(&points)));
        }
    }
    (lines, dashes, heads, labels)
}

fn distinct(points: &[Point]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for point in points {
        let same = out
            .last()
            .is_some_and(|last| (last.0 - point.0).abs() < 0.5 && (last.1 - point.1).abs() < 0.5);
        if !same {
            out.push(*point);
        }
    }
    out
}

fn unit(from: Point, to: Point) -> (Point, f32) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = dx.hypot(dy);
    if length < f32::EPSILON {
        return ((0.0, 0.0), 0.0);
    }
    ((dx / length, dy / length), length)
}

pub(super) fn polyline(mut node: PathNode, points: &[Point]) -> PathNode {
    let points = distinct(points);
    let Some(first) = points.first() else {
        return node;
    };
    node = node.move_to(first.0, first.1);
    for window in points.windows(3) {
        let &[before, corner, after] = window else {
            continue;
        };
        let ((ux, uy), into) = unit(before, corner);
        let ((vx, vy), out) = unit(corner, after);
        let r = CORNER.min(into / 2.0).min(out / 2.0);
        node = node
            .line_to((-ux).mul_add(r, corner.0), (-uy).mul_add(r, corner.1))
            .quad_to(
                corner.0,
                corner.1,
                vx.mul_add(r, corner.0),
                vy.mul_add(r, corner.1),
            );
    }
    match points.last() {
        Some(last) if points.len() > 1 => node.line_to(last.0, last.1),
        _ => node,
    }
}

pub(super) fn dashed_line(mut node: PathNode, points: &[Point]) -> PathNode {
    for pair in distinct(points).windows(2) {
        let &[start, finish] = pair else {
            continue;
        };
        let ((ux, uy), length) = unit(start, finish);
        let steps = std::iter::successors(Some(0.0_f32), |at| Some(at + DASH + DASH_GAP));
        for at in steps.take_while(|at| *at < length) {
            let end = (at + DASH).min(length);
            node = node
                .move_to(ux.mul_add(at, start.0), uy.mul_add(at, start.1))
                .line_to(ux.mul_add(end, start.0), uy.mul_add(end, start.1));
        }
    }
    node
}

fn arrow_head(node: PathNode, points: &mut [Point]) -> PathNode {
    let count = points.len();
    let (Some(tip), Some(before)) = (
        points.last().copied(),
        distinct(points).iter().rev().nth(1).copied(),
    ) else {
        return node;
    };
    let ((ux, uy), length) = unit(before, tip);
    let head = HEAD.min(length);
    let base = ((-ux).mul_add(head, tip.0), (-uy).mul_add(head, tip.1));
    if let Some(last) = points.get_mut(count.saturating_sub(1)) {
        *last = base;
    }
    barb(node, tip, base, (ux, uy), head * 0.55)
}

pub(super) fn barb(node: PathNode, tip: Point, base: Point, dir: Point, half: f32) -> PathNode {
    let (ux, uy) = dir;
    node.move_to(tip.0, tip.1)
        .line_to((-uy).mul_add(half, base.0), ux.mul_add(half, base.1))
        .line_to(uy.mul_add(half, base.0), (-ux).mul_add(half, base.1))
        .close()
}

fn middle(points: &[Point]) -> Point {
    let points = distinct(points);
    let count = points.len();
    let (Some(a), Some(b)) = (
        points.get(count.saturating_sub(1) / 2),
        points.get(count / 2),
    ) else {
        return (0.0, 0.0);
    };
    if count.is_multiple_of(2) {
        (f32::midpoint(a.0, b.0), f32::midpoint(a.1, b.1))
    } else {
        *a
    }
}

pub(super) fn edge_label(label: &str, at: Point) -> NodeBuilder {
    let width = 2.0f32.mul_add(space(2), cells(label.chars().count(), LABEL_CHAR));
    container()
        .layout(
            LayoutStyle::flex_row()
                .position(Position::Absolute)
                .inset_left(at.0 - width / 2.0)
                .inset_top(at.1 - LABEL_H / 2.0)
                .fixed_size(width, LABEL_H)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center),
        )
        .background(Token::Surface)
        .radius(Radius::Full)
        .border_width(1)
        .border_color(Token::BorderSubtle)
        .child(
            text(label.to_string())
                .class("text-xs")
                .text_color(Token::OnSurfaceMuted)
                .nowrap(),
        )
}
