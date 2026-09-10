use std::collections::VecDeque;

use super::rope::Rope;

#[derive(Debug, Clone)]
pub struct History {
    undo: VecDeque<Rope>,
    redo: VecDeque<Rope>,
    capacity: usize,
}

impl History {
    pub const DEFAULT_CAPACITY: usize = 1000;

    #[must_use]
    pub fn new(initial: Rope) -> Self {
        let mut undo = VecDeque::with_capacity(16);
        undo.push_back(initial);
        Self {
            undo,
            redo: VecDeque::new(),
            capacity: Self::DEFAULT_CAPACITY,
        }
    }

    #[must_use]
    pub fn bounded(initial: Rope, capacity: usize) -> Self {
        let cap = capacity.max(1);
        let mut undo = VecDeque::with_capacity(cap.min(16));
        undo.push_back(initial);
        Self {
            undo,
            redo: VecDeque::new(),
            capacity: cap,
        }
    }

    pub fn checkpoint(&mut self, current: &Rope) {
        if self.undo.back().is_some_and(|top| top.same_root(current)) {
            self.redo.clear();
            return;
        }
        self.undo.push_back(current.snapshot());
        self.redo.clear();
        while self.undo.len() > self.capacity {
            self.undo.pop_front();
        }
    }

    pub fn undo(&mut self, current: &mut Rope) -> bool {
        while self.undo.back().is_some_and(|top| top.same_root(current)) {
            self.undo.pop_back();
        }
        let Some(prev) = self.undo.pop_back() else {
            return false;
        };
        let snapshot_current = current.snapshot();
        *current = prev;
        self.redo.push_back(snapshot_current);
        while self.redo.len() > self.capacity {
            self.redo.pop_front();
        }
        true
    }

    pub fn redo(&mut self, current: &mut Rope) -> bool {
        let Some(next) = self.redo.pop_back() else {
            return false;
        };
        let snapshot_current = current.snapshot();
        *current = next;
        self.undo.push_back(snapshot_current);
        while self.undo.len() > self.capacity {
            self.undo.pop_front();
        }
        true
    }

    #[must_use]
    pub fn depth(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::missing_panics_doc
)]
#[path = "history_tests.rs"]
mod tests;
