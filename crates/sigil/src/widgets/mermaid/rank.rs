use std::collections::VecDeque;

use super::flow::Graph;

pub(super) fn rank(graph: &Graph) -> Vec<usize> {
    let back = back_edges(graph);
    let count = graph.nodes.len();
    let mut out_edges = vec![Vec::new(); count];
    let mut indegree = vec![0_usize; count];
    for (edge, is_back) in graph.edges.iter().zip(&back) {
        if *is_back || edge.from == edge.to || edge.to >= count {
            continue;
        }
        if let Some(list) = out_edges.get_mut(edge.from) {
            list.push(edge.to);
            if let Some(degree) = indegree.get_mut(edge.to) {
                *degree = degree.saturating_add(1);
            }
        }
    }
    let mut rank = vec![0_usize; count];
    let mut ready: VecDeque<usize> = (0..count)
        .filter(|node| indegree.get(*node) == Some(&0))
        .collect();
    while let Some(node) = ready.pop_front() {
        let want = rank.get(node).copied().unwrap_or(0).saturating_add(1);
        for target in out_edges.get(node).into_iter().flatten() {
            if let Some(slot) = rank.get_mut(*target) {
                *slot = (*slot).max(want);
            }
            if let Some(degree) = indegree.get_mut(*target) {
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    ready.push_back(*target);
                }
            }
        }
    }
    rank
}

fn back_edges(graph: &Graph) -> Vec<bool> {
    let count = graph.nodes.len();
    let mut adjacency = vec![Vec::new(); count];
    for (index, edge) in graph.edges.iter().enumerate() {
        if let Some(list) = adjacency.get_mut(edge.from) {
            list.push(index);
        }
    }
    let mut colour = vec![0_u8; count];
    let mut back = vec![false; graph.edges.len()];
    for start in 0..count {
        if colour.get(start) == Some(&0) {
            visit(graph, &adjacency, start, &mut colour, &mut back);
        }
    }
    back
}

fn visit(
    graph: &Graph,
    adjacency: &[Vec<usize>],
    start: usize,
    colour: &mut [u8],
    back: &mut [bool],
) {
    let mut stack = vec![(start, 0_usize)];
    if let Some(slot) = colour.get_mut(start) {
        *slot = 1;
    }
    while let Some((node, next)) = stack.last_mut() {
        let Some(index) = adjacency
            .get(*node)
            .and_then(|list| list.get(*next))
            .copied()
        else {
            if let Some(slot) = colour.get_mut(*node) {
                *slot = 2;
            }
            stack.pop();
            continue;
        };
        *next = next.saturating_add(1);
        let Some(target) = graph.edges.get(index).map(|edge| edge.to) else {
            continue;
        };
        match colour.get(target).copied() {
            Some(1) => {
                if let Some(flag) = back.get_mut(index) {
                    *flag = true;
                }
            }
            Some(0) => {
                if let Some(slot) = colour.get_mut(target) {
                    *slot = 1;
                }
                stack.push((target, 0));
            }
            _ => {}
        }
    }
}
