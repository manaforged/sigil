use super::flow_layout::expand;
use super::flow_parse::parse;

pub(super) const GAP_ROWS: usize = 3;
pub(super) const GAP_COLS: usize = 5;
pub(super) const SPACING: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Shape {
    Box,
    Round,
    Stadium,
    Circle,
    Diamond,
    Hexagon,
    Cylinder,
    Subroutine,
    Flag,
    Terminal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Dir {
    Down,
    Up,
    Right,
    Left,
}

impl Dir {
    pub(super) const fn vertical(self) -> bool {
        matches!(self, Self::Down | Self::Up)
    }

    pub(super) const fn forward(self) -> bool {
        matches!(self, Self::Down | Self::Right)
    }
}

pub(super) struct Node {
    pub(super) id: String,
    pub(super) label: Vec<String>,
    pub(super) shape: Shape,
    pub(super) dummy: bool,
    pub(super) dashed: bool,
}

pub(super) struct Edge {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) label: String,
    pub(super) dashed: bool,
    pub(super) arrow: bool,
}

pub(super) struct Graph {
    pub(super) nodes: Vec<Node>,
    pub(super) edges: Vec<Edge>,
    pub(super) dir: Dir,
    pub(super) state: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) w: usize,
    pub(super) h: usize,
}

pub(super) fn draw(source: &str) -> Option<Vec<String>> {
    let graph = parse(source)?;
    if graph.nodes.is_empty() {
        return None;
    }
    Some(render(graph))
}

pub(super) fn render(graph: Graph) -> Vec<String> {
    let ranks = graph.fold(graph.rank());
    let (graph, ranks) = expand(graph, ranks);
    let layers = graph.order(&ranks);
    let rects = graph.place(&ranks, &layers);
    graph.paint(&rects)
}
