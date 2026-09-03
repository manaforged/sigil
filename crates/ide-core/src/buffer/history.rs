//! Undo/redo history over rope snapshots.
//!
//! # Model
//!
//! A `History` owns two stacks of cheap `Rope` snapshots: the undo
//! stack (states older than the current buffer) and the redo stack
//! (states newer than the current buffer, populated by `undo`).
//!
//! Each snapshot is `Rope::snapshot()` — an O(1) `Arc`-clone of the
//! root node. Memory cost per snapshot is the per-edit copy-on-write
//! delta, not the full buffer.
//!
//! # Coalescing
//!
//! `History` does NOT auto-checkpoint on every edit. The caller
//! decides when an edit-group ends — typical triggers in an editor:
//! 500ms idle, word boundary, paste, focus loss, save. Each call to
//! [`History::checkpoint`] pushes the current state onto the undo
//! stack and clears the redo stack (the new edit branches history).
//!
//! # Depth cap
//!
//! Bounded by [`History::DEFAULT_CAPACITY`] (1000) snapshots. When
//! the undo stack overflows, the oldest snapshot is dropped. The
//! redo stack is bounded by the same capacity but is naturally
//! capped by the undo depth that produced it.

use std::collections::VecDeque;

use super::rope::Rope;

/// Bounded undo/redo history of `Rope` snapshots.
///
/// Snapshots are O(1) `Arc`-clones; the memory cost per checkpoint
/// is the copy-on-write delta from the previous tree.
#[derive(Debug, Clone)]
pub struct History {
    /// States older than the current buffer (top = most recent).
    undo: VecDeque<Rope>,
    /// States newer than the current buffer (top = next redo).
    redo: VecDeque<Rope>,
    /// Maximum number of snapshots retained per stack.
    capacity: usize,
}

impl History {
    /// Default upper bound on retained snapshots per stack.
    pub const DEFAULT_CAPACITY: usize = 1000;

    /// Create a history seeded with `initial` as the baseline.
    ///
    /// The baseline is the state the buffer was in before any
    /// tracked edit. The first `checkpoint` will push the *current*
    /// edited rope on top of this baseline.
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

    /// Create a history with a custom depth cap.
    ///
    /// `capacity` is clamped to at least 1 — a zero-capacity history
    /// could not represent even the baseline state.
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

    /// Push `current` onto the undo stack and clear the redo stack.
    ///
    /// Call this when an edit-group ends (idle pause, word boundary,
    /// paste, save). The snapshot is O(1); the redo clear reflects
    /// that a new edit branches history.
    ///
    /// If the top of the undo stack already shares the root `Arc`
    /// with `current`, the push is skipped — a duplicate checkpoint
    /// (no edits since the last call) is pure memory waste and would
    /// silently turn one logical undo into two physical undos.
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

    /// Restore the previous snapshot into `current`, returning
    /// `true` if a snapshot was applied.
    ///
    /// The pre-undo current state is pushed onto the redo stack so
    /// a matching `redo` call rolls forward. Pointer-equal entries
    /// at the top of the undo stack (created by a `checkpoint` call
    /// made *after* the most recent edit — a common editor pattern)
    /// are skipped so that one user-visible undo restores one
    /// user-visible edit group.
    pub fn undo(&mut self, current: &mut Rope) -> bool {
        // Skip pointer-equal entries at the top — these are
        // checkpoints of the very state we're already in.
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

    /// Restore the next snapshot (matching a prior `undo`) into
    /// `current`, returning `true` if a snapshot was applied.
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

    /// Return `(undo_count, redo_count)`.
    ///
    /// `undo_count` includes the baseline, so a freshly constructed
    /// history reports `(1, 0)`. A history with one checkpoint
    /// reports `(2, 0)`.
    #[must_use]
    pub fn depth(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }

    /// Maximum snapshots retained per stack.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
}

// ---- Tests -------------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::missing_panics_doc
)]
#[path = "history_tests.rs"]
mod tests;
