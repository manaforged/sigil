use super::flow_layout::expand;
use super::flow_parse::parse;

pub(super) const GAP_ROWS: usize = 3;
pub(super) const GAP_COLS: usize = 5;
pub(super) const SPACING: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NodeShape {
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

pub(super) struct FlowNode {
    pub(super) id: String,
    pub(super) label: Vec<String>,
    pub(super) shape: NodeShape,
    pub(super) dummy: bool,
    pub(super) dashed: bool,
}

pub(super) struct FlowEdge {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) label: String,
    pub(super) dashed: bool,
    pub(super) arrow: bool,
}

pub(super) struct Graph {
    pub(super) nodes: Vec<FlowNode>,
    pub(super) edges: Vec<FlowEdge>,
    pub(super) dir: Dir,
    pub(super) state: bool,
}

#[derive(Clone, Copy)]
pub(super) struct GridRect {
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
    let (graph, rects) = arrange(graph);
    graph.paint(&rects)
}

pub(super) fn arrange(graph: Graph) -> (Graph, Vec<GridRect>) {
    let ranks = graph.fold(graph.rank());
    let (graph, ranks) = expand(graph, ranks);
    let layers = graph.order(&ranks);
    let rects = graph.place(&ranks, &layers);
    (graph, rects)
}
