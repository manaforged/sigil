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

fn offsets(layers: &[Vec<usize>], rank_size: &[usize], gap: usize, forward: bool) -> Vec<usize> {
    let count = layers.len();
    let mut offsets = vec![0_usize; count];
    let mut cursor = 0_usize;
    for step in 0..count {
        let rank = if forward {
            step
        } else {
            count.saturating_sub(1).saturating_sub(step)
        };
        if let Some(slot) = offsets.get_mut(rank) {
            *slot = cursor;
        }
        cursor = cursor
            .saturating_add(rank_size.get(rank).copied().unwrap_or(1))
            .saturating_add(gap);
    }
    offsets
}

impl Graph {
    pub(super) fn order(&self, ranks: &[usize]) -> Vec<Vec<usize>> {
        let depth = ranks.iter().max().map_or(0, |rank| rank.saturating_add(1));
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); depth];
        for (node, rank) in ranks.iter().enumerate() {
            if let Some(layer) = layers.get_mut(*rank) {
                layer.push(node);
            }
        }
        for _ in 0..3 {
            for rank in 1..depth {
                self.sweep(&mut layers, rank, true);
            }
            for rank in (0..depth.saturating_sub(1)).rev() {
                self.sweep(&mut layers, rank, false);
            }
        }
        layers
    }

    fn sweep(&self, layers: &mut [Vec<usize>], rank: usize, down: bool) {
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
            .map(|(index, node)| (self.barycenter(*node, &positions, down, index), *node))
            .collect();
        keyed.sort_by_key(|(key, _)| *key);
        if let Some(slot) = layers.get_mut(rank) {
            *slot = keyed.into_iter().map(|(_, node)| node).collect();
        }
    }

    fn barycenter(
        &self,
        node: usize,
        positions: &HashMap<usize, usize>,
        down: bool,
        index: usize,
    ) -> usize {
        let mut sum = 0_usize;
        let mut count = 0_usize;
        for edge in &self.edges {
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

    pub(super) fn place(&self, ranks: &[usize], layers: &[Vec<usize>]) -> Vec<Rect> {
        let sizes: Vec<(usize, usize)> = self.nodes.iter().map(size).collect();
        let vertical = self.dir.vertical();
        let oriented = |node: usize| {
            let (w, h) = sizes.get(node).copied().unwrap_or((1, 1));
            if vertical { (h, w) } else { (w, h) }
        };
        let across = |node: usize| oriented(node).1;
        let rank_size: Vec<usize> = layers
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|node| oriented(*node).0)
                    .max()
                    .unwrap_or(1)
            })
            .collect();
        let gap = if vertical { GAP_ROWS } else { GAP_COLS };
        let offsets = offsets(layers, &rank_size, gap, self.dir.forward());
        let positions = self.positions(layers, &across);
        self.nodes
            .iter()
            .enumerate()
            .map(|(node, entry)| {
                let rank = ranks.get(node).copied().unwrap_or(0);
                let offset = offsets.get(rank).copied().unwrap_or(0);
                let span = rank_size.get(rank).copied().unwrap_or(1);
                let at = positions.get(node).copied().unwrap_or(0);
                let (long, wide) = oriented(node);
                let long = if entry.dummy { span } else { long };
                let ((x, w), (y, h)) = if vertical {
                    ((at, wide), (offset, long))
                } else {
                    ((offset, long), (at, wide))
                };
                Rect { x, y, w, h }
            })
            .collect()
    }

    fn positions(&self, layers: &[Vec<usize>], across: &dyn Fn(usize) -> usize) -> Vec<usize> {
        let mut positions = vec![0_usize; self.nodes.len()];
        for layer in layers {
            let mut cursor = 0_usize;
            for node in layer {
                let extent = across(*node);
                let desired = self
                    .parents(*node, &positions, across)
                    .map_or(cursor, |centre| centre.saturating_sub(extent / 2));
                let at = desired.max(cursor);
                if let Some(slot) = positions.get_mut(*node) {
                    *slot = at;
                }
                cursor = at.saturating_add(extent).saturating_add(SPACING);
            }
        }
        compact(layers, &mut positions, across);
        positions
    }

    fn parents(
        &self,
        node: usize,
        positions: &[usize],
        across: &dyn Fn(usize) -> usize,
    ) -> Option<usize> {
        let centres: Vec<usize> = self
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
}

fn compact(layers: &[Vec<usize>], positions: &mut [usize], across: &dyn Fn(usize) -> usize) {
    let span = |layer: &Vec<usize>| {
        layer
            .iter()
            .map(|node| across(*node).saturating_add(SPACING))
            .sum::<usize>()
            .saturating_sub(SPACING)
    };
    let widest = layers.iter().map(span).max().unwrap_or(0);
    for layer in layers {
        let left = layer
            .iter()
            .filter_map(|node| positions.get(*node).copied())
            .min()
            .unwrap_or(0);
        let right = layer
            .iter()
            .filter_map(|node| {
                positions
                    .get(*node)
                    .map(|at| at.saturating_add(across(*node)))
            })
            .max()
            .unwrap_or(0);
        let shift = left.min(right.saturating_sub(widest));
        for node in layer {
            if let Some(at) = positions.get_mut(*node) {
                *at = at.saturating_sub(shift);
            }
        }
    }
}
