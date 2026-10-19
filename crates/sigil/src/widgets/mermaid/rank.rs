use std::collections::VecDeque;

use super::flow::{Graph, SPACING};
use super::flow_layout::size;

const FOLD_COLS: usize = 72;

impl Graph {
    pub(super) fn rank(&self) -> Vec<usize> {
        let back = self.back_edges();
        let count = self.nodes.len();
        let mut out_edges = vec![Vec::new(); count];
        let mut indegree = vec![0_usize; count];
        for (edge, is_back) in self.edges.iter().zip(&back) {
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

    pub(super) fn fold(&self, mut ranks: Vec<usize>) -> Vec<usize> {
        if !self.dir.vertical() {
            return ranks;
        }
        let mut rank = 0_usize;
        while ranks.iter().any(|at| *at >= rank) {
            let mut used = 0_usize;
            let mut spill = Vec::new();
            for (index, (node, at)) in self.nodes.iter().zip(&ranks).enumerate() {
                if *at != rank {
                    continue;
                }
                let width = size(node).0;
                let need = if used == 0 {
                    width
                } else {
                    used.saturating_add(SPACING).saturating_add(width)
                };
                if !spill.is_empty() || (used > 0 && need > FOLD_COLS) {
                    spill.push(index);
                } else {
                    used = need;
                }
            }
            if !spill.is_empty() {
                for at in &mut ranks {
                    if *at > rank {
                        *at = at.saturating_add(1);
                    }
                }
                for index in spill {
                    if let Some(at) = ranks.get_mut(index) {
                        *at = rank.saturating_add(1);
                    }
                }
            }
            rank = rank.saturating_add(1);
        }
        ranks
    }

    fn back_edges(&self) -> Vec<bool> {
        let count = self.nodes.len();
        let mut adjacency = vec![Vec::new(); count];
        for (index, edge) in self.edges.iter().enumerate() {
            if let Some(list) = adjacency.get_mut(edge.from) {
                list.push(index);
            }
        }
        let mut colour = vec![0_u8; count];
        let mut back = vec![false; self.edges.len()];
        for start in 0..count {
            if colour.get(start) == Some(&0) {
                self.visit(&adjacency, start, &mut colour, &mut back);
            }
        }
        back
    }

    fn visit(&self, adjacency: &[Vec<usize>], start: usize, colour: &mut [u8], back: &mut [bool]) {
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
            let Some(target) = self.edges.get(index).map(|edge| edge.to) else {
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
}
