use std::collections::HashMap;

use super::flow::{Edge, GAP_COLS, GAP_ROWS, Graph, Node, Rect, SPACING, Shape};

pub(super) fn expand(mut graph: Graph, mut ranks: Vec<usize>) -> (Graph, Vec<usize>) {
    let mut edges = Vec::new();
    for edge in std::mem::take(&mut graph.edges) {
        let from_rank = ranks.get(edge.from).copied().unwrap_or(0);
        let to_rank = ranks.get(edge.to).copied().unwrap_or(0);
        if to_rank <= from_rank.saturating_add(1) {
            edges.push(edge);
            continue;
        }
        let mut previous = edge.from;
        for rank in from_rank.saturating_add(1)..to_rank {
            graph.nodes.push(Node {
                id: String::new(),
                label: Vec::new(),
                shape: Shape::Box,
                dummy: true,
                dashed: edge.dashed,
            });
            ranks.push(rank);
            let dummy = graph.nodes.len().saturating_sub(1);
            edges.push(Edge {
                from: previous,
                to: dummy,
                label: if previous == edge.from {
                    edge.label.clone()
                } else {
                    String::new()
                },
                dashed: edge.dashed,
                arrow: false,
            });
            previous = dummy;
        }
        edges.push(Edge {
            from: previous,
            to: edge.to,
            label: String::new(),
            dashed: edge.dashed,
            arrow: edge.arrow,
        });
    }
    graph.edges = edges;
    (graph, ranks)
}

pub(super) fn order(graph: &Graph, ranks: &[usize]) -> Vec<Vec<usize>> {
    let depth = ranks.iter().max().map_or(0, |rank| rank.saturating_add(1));
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); depth];
    for (node, rank) in ranks.iter().enumerate() {
        if let Some(layer) = layers.get_mut(*rank) {
            layer.push(node);
        }
    }
    for _ in 0..3 {
        for rank in 1..depth {
            sweep(graph, &mut layers, rank, true);
        }
        for rank in (0..depth.saturating_sub(1)).rev() {
            sweep(graph, &mut layers, rank, false);
        }
    }
    layers
}

pub(super) fn sweep(graph: &Graph, layers: &mut [Vec<usize>], rank: usize, down: bool) {
    let neighbour = if down {
        rank.saturating_sub(1)
    } else {
        rank.saturating_add(1)
    };
    let positions: HashMap<usize, usize> = layers
        .get(neighbour)
        .map(|layer| {
            layer
                .iter()
                .enumerate()
                .map(|(index, node)| (*node, index))
                .collect()
        })
        .unwrap_or_default();
    let Some(layer) = layers.get(rank).cloned() else {
        return;
    };
    let mut keyed: Vec<(usize, usize)> = layer
        .iter()
        .enumerate()
        .map(|(index, node)| (barycenter(graph, *node, &positions, down, index), *node))
        .collect();
    keyed.sort_by_key(|(key, _)| *key);
    if let Some(slot) = layers.get_mut(rank) {
        slot.clone_from(&keyed.into_iter().map(|(_, node)| node).collect());
    }
}

pub(super) fn barycenter(
    graph: &Graph,
    node: usize,
    positions: &HashMap<usize, usize>,
    down: bool,
    index: usize,
) -> usize {
    let mut sum = 0_usize;
    let mut count = 0_usize;
    for edge in &graph.edges {
        let other = match (down, edge.to == node, edge.from == node) {
            (true, true, _) => edge.from,
            (false, _, true) => edge.to,
            _ => continue,
        };
        if let Some(position) = positions.get(&other) {
            sum = sum.saturating_add(position.saturating_mul(1000));
            count = count.saturating_add(1);
        }
    }
    sum.checked_div(count)
        .unwrap_or_else(|| index.saturating_mul(1000))
}

pub(super) fn size(node: &Node) -> (usize, usize) {
    if node.dummy {
        return (1, 1);
    }
    let text = node
        .label
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let extra = match node.shape {
        Shape::Circle | Shape::Stadium | Shape::Subroutine | Shape::Flag => 2,
        _ => 0,
    };
    (
        text.saturating_add(extra).saturating_add(4),
        node.label.len().max(1).saturating_add(2),
    )
}

pub(super) fn place(graph: &Graph, ranks: &[usize], layers: &[Vec<usize>]) -> Vec<Rect> {
    let sizes: Vec<(usize, usize)> = graph.nodes.iter().map(size).collect();
    let vertical = graph.dir.vertical();
    let along = |node: usize| {
        let (w, h) = sizes.get(node).copied().unwrap_or((1, 1));
        if vertical { h } else { w }
    };
    let across = |node: usize| {
        let (w, h) = sizes.get(node).copied().unwrap_or((1, 1));
        if vertical { w } else { h }
    };
    let rank_size: Vec<usize> = layers
        .iter()
        .map(|layer| layer.iter().map(|node| along(*node)).max().unwrap_or(1))
        .collect();
    let gap = if vertical { GAP_ROWS } else { GAP_COLS };
    let offsets = offsets(layers, &rank_size, gap, graph.dir.forward());
    let positions = positions(graph, layers, &across);
    graph
        .nodes
        .iter()
        .enumerate()
        .map(|(node, entry)| {
            let (w, h) = sizes.get(node).copied().unwrap_or((1, 1));
            let rank = ranks.get(node).copied().unwrap_or(0);
            let offset = offsets.get(rank).copied().unwrap_or(0);
            let span = rank_size.get(rank).copied().unwrap_or(1);
            let at = positions.get(node).copied().unwrap_or(0);
            if vertical {
                Rect {
                    x: at,
                    y: offset,
                    w,
                    h: if entry.dummy { span } else { h },
                }
            } else {
                Rect {
                    x: offset,
                    y: at,
                    w: if entry.dummy { span } else { w },
                    h,
                }
            }
        })
        .collect()
}

pub(super) fn offsets(
    layers: &[Vec<usize>],
    rank_size: &[usize],
    gap: usize,
    forward: bool,
) -> Vec<usize> {
    let mut offsets = vec![0_usize; layers.len()];
    let mut cursor = 0_usize;
    let order: Vec<usize> = if forward {
        (0..layers.len()).collect()
    } else {
        (0..layers.len()).rev().collect()
    };
    for rank in order {
        if let Some(slot) = offsets.get_mut(rank) {
            slot.clone_from(&cursor);
        }
        cursor = cursor
            .saturating_add(rank_size.get(rank).copied().unwrap_or(1))
            .saturating_add(gap);
    }
    offsets
}

pub(super) fn positions(
    graph: &Graph,
    layers: &[Vec<usize>],
    across: &dyn Fn(usize) -> usize,
) -> Vec<usize> {
    let mut positions = vec![0_usize; graph.nodes.len()];
    for layer in layers {
        let mut cursor = 0_usize;
        for node in layer {
            let extent = across(*node);
            let desired = parents(graph, *node, &positions, across)
                .map_or(cursor, |centre| centre.saturating_sub(extent / 2));
            let at = desired.max(cursor);
            if let Some(slot) = positions.get_mut(*node) {
                slot.clone_from(&at);
            }
            cursor = at.saturating_add(extent).saturating_add(SPACING);
        }
    }
    positions
}

pub(super) fn parents(
    graph: &Graph,
    node: usize,
    positions: &[usize],
    across: &dyn Fn(usize) -> usize,
) -> Option<usize> {
    let centres: Vec<usize> = graph
        .edges
        .iter()
        .filter(|edge| edge.to == node && edge.from != node)
        .filter_map(|edge| {
            let at = positions.get(edge.from)?;
            Some(at.saturating_add(across(edge.from) / 2))
        })
        .collect();
    if centres.is_empty() {
        return None;
    }
    Some(centres.iter().sum::<usize>() / centres.len())
}
