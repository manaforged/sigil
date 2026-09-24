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

pub(crate) fn line(value: f32) -> f32 {
    cell().unwrap_or(value)
}

pub(crate) fn detail(value: f32) -> f32 {
    cell().map_or(value, |_| 0.0)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CellForm<T> {
    window: T,
    cell: T,
}

impl<T> CellForm<T> {
    pub(crate) const fn new(window: T, cell: T) -> Self {
        Self { window, cell }
    }

    pub(crate) fn pick(self) -> T {
        if cell().is_some() {
            self.cell
        } else {
            self.window
        }
    }
}

pub(crate) fn track(value: f32) -> f32 {
    let Some(cell) = cell() else {
        return value;
    };
    (value / cell).round().max(1.0) * cell
}

pub(crate) fn gap(value: f32) -> f32 {
    let Some(cell) = cell() else {
        return value;
    };
    if value <= 0.0 {
        return value;
    }
    (value / cell).round() * cell
}

pub(crate) fn row_gap(value: f32) -> f32 {
    let Some(cell) = cell() else {
        return value;
    };
    if value <= 0.0 {
        return value;
    }
    if value < cell {
        return 0.0;
    }
    (value / cell).round() * cell
}

pub(crate) fn ruled(value: f32) -> f32 {
    cell().map_or(value, |cell| cell * 2.0)
}

pub(crate) fn size(value: Option<f32>) -> Option<f32> {
    value.map(span)
}

pub(crate) fn rule_edges(edges: Edges, height: Option<f32>) -> Edges {
    let Some(cell) = cell() else {
        return edges;
    };
    if height.is_some_and(|height| height <= cell) {
        return Edges {
            left: span(edges.left),
            right: span(edges.right),
            top: span(edges.top),
            bottom: span(edges.bottom),
        };
    }
    Edges {
        left: rule(edges.left),
        right: rule(edges.right),
        top: rule(edges.top),
        bottom: rule(edges.bottom),
    }
}

pub(crate) fn rule(value: f32) -> f32 {
    if value <= 0.0 { value } else { track(value) }
}

pub(crate) fn rect(x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
    let Some(cell) = cell() else {
        return (x, y, w, h);
    };
    let edge = |start: f32, extent: f32| {
        if extent < cell {
            return (start, extent);
        }
        let low = (start / cell).round() * cell;
        let high = ((start + extent) / cell).round() * cell;
        (low, high - low)
    };
    let (x, w) = edge(x, w);
    let (y, h) = edge(y, h);
    (x, y, w, h)
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
