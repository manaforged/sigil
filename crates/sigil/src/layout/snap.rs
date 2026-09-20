use crate::layout::resolve::{AutoEdges, Edges};

fn cell() -> Option<f32> {
    crate::style::cell_size().filter(|cell| *cell > 0.0)
}

pub(crate) fn span(value: f32) -> f32 {
    let Some(cell) = cell() else {
        return value;
    };
    if value <= 0.0 || value < cell {
        return value;
    }
    (value / cell).round() * cell
}

pub(crate) fn gap(value: f32) -> f32 {
    let Some(cell) = cell() else {
        return value;
    };
    if value <= 0.0 {
        return value;
    }
    (value / cell).round().max(1.0) * cell
}

pub(crate) fn size(value: Option<f32>) -> Option<f32> {
    value.map(span)
}

pub(crate) fn rule_edges(edges: Edges) -> Edges {
    if cell().is_none() {
        return edges;
    }
    Edges {
        left: span(edges.left),
        right: span(edges.right),
        top: span(edges.top),
        bottom: span(edges.bottom),
    }
}

pub(crate) fn gap_edges(edges: Edges) -> Edges {
    if cell().is_none() {
        return edges;
    }
    Edges {
        left: gap(edges.left),
        right: gap(edges.right),
        top: gap(edges.top),
        bottom: gap(edges.bottom),
    }
}

pub(crate) fn gap_auto_edges(edges: AutoEdges) -> AutoEdges {
    if cell().is_none() {
        return edges;
    }
    AutoEdges {
        left: edges.left.map(gap),
        right: edges.right.map(gap),
        top: edges.top.map(gap),
        bottom: edges.bottom.map(gap),
    }
}
