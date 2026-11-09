use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Position};
use crate::style::{BORDER_WIDTH, Paint, Radius, Token, space};
use crate::widgets::Widget;
use crate::widgets::vector::{PathNode, path};

use super::flow::{Graph, GridRect, NodeShape, arrange};
use super::flow_parse::parse;

mod route;

use route::{arrow_head, middle, route};
pub(in crate::widgets::mermaid) use route::{barb, dashed_line, polyline};

pub(super) const CELL_W: f32 = 7.0;
pub(super) const CELL_H: f32 = space(4);
const CORNER: f32 = 8.0;
const HEAD: f32 = 7.0;
const DASH: f32 = 4.0;
const DASH_GAP: f32 = 3.0;
const DOT: f32 = 6.0;
const OVERLAP: f32 = 12.0;
const LANE: f32 = space(4);
const LABEL_H: f32 = 20.0;
const LABEL_CHAR: f32 = 7.0;
const CIRCLE_K: f32 = 0.552_284_8;

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

fn shapes(graph: &Graph, areas: &[Area], size: (f32, f32)) -> PathNode {
    let mut node = canvas(size);
    for (entry, area) in graph.nodes.iter().zip(areas) {
        if !entry.dummy && entry.shape != NodeShape::Terminal {
            node = shape(node, entry.shape, *area);
        }
    }
    node.fill(Token::Surface)
        .stroke(Token::Border, f32::from(BORDER_WIDTH))
}

fn dots(graph: &Graph, areas: &[Area], size: (f32, f32)) -> PathNode {
    let mut node = canvas(size);
    for (entry, area) in graph.nodes.iter().zip(areas) {
        if entry.shape == NodeShape::Terminal {
            let end = entry.label.first().is_some_and(|glyph| glyph == "◉");
            node = ellipse(node, area.cx(), area.cy(), DOT, DOT);
            if end {
                node = ellipse(node, area.cx(), area.cy(), DOT * 0.55, DOT * 0.55);
            }
        }
    }
    node.fill(Token::OnSurface)
}

fn shape(node: PathNode, shape: NodeShape, area: Area) -> PathNode {
    match shape {
        NodeShape::Round => rounded(node, area, CORNER * 1.5),
        NodeShape::Stadium | NodeShape::Terminal => rounded(node, area, area.h / 2.0),
        NodeShape::Circle => ellipse(node, area.cx(), area.cy(), area.w / 2.0, area.h / 2.0),
        NodeShape::Diamond | NodeShape::Hexagon => hexagon(node, area),
        NodeShape::Cylinder => cylinder(node, area),
        NodeShape::Subroutine => subroutine(node, area),
        NodeShape::Flag => flag(node, area),
        NodeShape::Box => rounded(node, area, CORNER / 2.0),
    }
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

fn ellipse(node: PathNode, cx: f32, cy: f32, rx: f32, ry: f32) -> PathNode {
    let (kx, ky) = (rx * CIRCLE_K, ry * CIRCLE_K);
    node.move_to(cx + rx, cy)
        .cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry)
        .cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy)
        .cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry)
        .cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy)
        .close()
}

fn hexagon(node: PathNode, area: Area) -> PathNode {
    let inset = (area.h / 2.0).min(area.w / 4.0);
    node.move_to(area.x + inset, area.y)
        .line_to(area.right() - inset, area.y)
        .line_to(area.right(), area.cy())
        .line_to(area.right() - inset, area.bottom())
        .line_to(area.x + inset, area.bottom())
        .line_to(area.x, area.cy())
        .close()
}

fn cylinder(node: PathNode, area: Area) -> PathNode {
    let lip = (area.h / 6.0).min(6.0);
    let (x0, x1, top, bottom) = (area.x, area.right(), area.y + lip, area.bottom() - lip);
    let k = CIRCLE_K * area.w / 2.0;
    let belly = 2.0f32.mul_add(lip, top);
    node.move_to(x0, top)
        .cubic_to(x0, top - lip, area.cx() - k, area.y, area.cx(), area.y)
        .cubic_to(area.cx() + k, area.y, x1, top - lip, x1, top)
        .line_to(x1, bottom)
        .cubic_to(
            x1,
            bottom + lip,
            area.cx() + k,
            area.bottom(),
            area.cx(),
            area.bottom(),
        )
        .cubic_to(area.cx() - k, area.bottom(), x0, bottom + lip, x0, bottom)
        .close()
        .move_to(x0, top)
        .cubic_to(x0, top + lip, area.cx() - k, belly, area.cx(), belly)
        .cubic_to(area.cx() + k, belly, x1, top + lip, x1, top)
}

fn subroutine(node: PathNode, area: Area) -> PathNode {
    let side = CELL_W;
    rounded(node, area, CORNER / 2.0)
        .move_to(area.x + side, area.y)
        .line_to(area.x + side, area.bottom())
        .move_to(area.right() - side, area.y)
        .line_to(area.right() - side, area.bottom())
}

fn flag(node: PathNode, area: Area) -> PathNode {
    node.move_to(area.x, area.y)
        .line_to(area.right(), area.y)
        .line_to(area.right(), area.bottom())
        .line_to(area.x, area.bottom())
        .line_to(area.x + CELL_W, area.cy())
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
        .bg(Token::Surface)
        .rounded(Radius::Full)
        .border_w(1)
        .border_color(Token::BorderSubtle)
        .child(
            text(label.to_string())
                .class("text-xs")
                .text_color(Token::OnSurfaceMuted)
                .nowrap(),
        )
}
