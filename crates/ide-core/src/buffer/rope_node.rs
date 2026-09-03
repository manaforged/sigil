//! Internal rope-tree nodes.
//!
//! The rope is a B-tree of UTF-8 leaves. Each node caches the
//! cumulative byte / char / line / UTF-16 counts of the subtree
//! rooted at it so all coordinate lookups are O(log n).
//!
//! Mutation is copy-on-write through `Arc::make_mut`. A clone of the
//! root `Arc<Node>` is an O(1) snapshot (the substrate for I-T3
//! history).

use std::sync::Arc;

use super::coords;

/// Target leaf byte length. Splits aim at this size so post-insert
/// leaves stay close to half-full and well above the minimum.
pub const LEAF_TARGET: usize = 512;
/// Maximum leaf byte length. Above this, a leaf splits on the next
/// edit that visits it.
pub const LEAF_MAX: usize = 1024;
/// Maximum number of children in an internal node.
pub const BRANCH_MAX: usize = 16;
/// Minimum number of children in an internal node (root excepted).
pub const BRANCH_MIN: usize = 4;

/// A rope tree node.
#[derive(Debug, Clone)]
pub enum Node {
    /// A leaf storing up to `LEAF_MAX` bytes of UTF-8.
    Leaf(Leaf),
    /// An internal node holding 1..=`BRANCH_MAX` children.
    Internal(Internal),
}

/// Cached counts for a subtree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// UTF-8 byte length.
    pub bytes: usize,
    /// Unicode scalar count.
    pub chars: usize,
    /// `\n` count.
    pub lines: usize,
    /// UTF-16 code-unit length.
    pub utf16: usize,
}

impl Counts {
    /// Counts for a UTF-8 slice.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.len(),
            chars: coords::count_chars(bytes),
            lines: coords::count_lines(bytes),
            utf16: coords::count_utf16(bytes),
        }
    }

    /// Sum two count vectors.
    #[must_use]
    pub const fn add(self, rhs: Self) -> Self {
        Self {
            bytes: self.bytes + rhs.bytes,
            chars: self.chars + rhs.chars,
            lines: self.lines + rhs.lines,
            utf16: self.utf16 + rhs.utf16,
        }
    }
}

/// Leaf — holds a small UTF-8 chunk and its counts.
#[derive(Debug, Clone)]
pub struct Leaf {
    /// Raw UTF-8 bytes.
    pub text: String,
    /// Cached counts.
    pub counts: Counts,
}

impl Leaf {
    /// Build a leaf from a UTF-8 string.
    #[must_use]
    pub fn from_string(text: String) -> Self {
        let counts = Counts::of(text.as_bytes());
        Self { text, counts }
    }

    /// Recompute cached counts. Call after mutating `text`.
    pub fn refresh(&mut self) {
        self.counts = Counts::of(self.text.as_bytes());
    }
}

/// Internal node — children + aggregated counts.
#[derive(Debug, Clone)]
pub struct Internal {
    /// 1..=`BRANCH_MAX` child subtrees.
    pub children: Vec<Arc<Node>>,
    /// Aggregated counts across `children`.
    pub counts: Counts,
}

impl Internal {
    /// Build from a child list and aggregate counts.
    #[must_use]
    pub fn from_children(children: Vec<Arc<Node>>) -> Self {
        let counts = children
            .iter()
            .map(|c| c.counts())
            .fold(Counts::default(), Counts::add);
        Self { children, counts }
    }
}

impl Node {
    /// Empty rope-tree (one empty leaf).
    #[must_use]
    pub fn empty() -> Arc<Self> {
        Arc::new(Self::Leaf(Leaf::from_string(String::new())))
    }

    /// Aggregated counts for this subtree.
    #[must_use]
    pub const fn counts(&self) -> Counts {
        match self {
            Self::Leaf(l) => l.counts,
            Self::Internal(i) => i.counts,
        }
    }

    /// Number of UTF-8 bytes in this subtree.
    #[must_use]
    pub const fn len_bytes(&self) -> usize {
        self.counts().bytes
    }

    /// Whether this node is a leaf.
    #[must_use]
    pub const fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    /// Depth of this subtree (a single leaf has depth 0).
    #[must_use]
    pub fn depth(&self) -> usize {
        match self {
            Self::Leaf(_) => 0,
            Self::Internal(i) => 1 + i.children.first().map_or(0, |c| c.depth()),
        }
    }
}

/// Build a balanced rope tree from a list of leaves.
///
/// Bottom-up: groups leaves into internal nodes of up to `BRANCH_MAX`
/// children, then groups those internals, etc. Returns the root.
#[must_use]
pub fn build_from_leaves(leaves: Vec<Leaf>) -> Arc<Node> {
    if leaves.is_empty() {
        return Node::empty();
    }
    let mut level: Vec<Arc<Node>> = leaves
        .into_iter()
        .map(|l| Arc::new(Node::Leaf(l)))
        .collect();
    while level.len() > 1 {
        level = group_level(level);
    }
    level.pop().unwrap_or_else(Node::empty)
}

fn group_level(nodes: Vec<Arc<Node>>) -> Vec<Arc<Node>> {
    let mut next: Vec<Arc<Node>> = Vec::with_capacity(nodes.len().div_ceil(BRANCH_MAX));
    let mut chunk: Vec<Arc<Node>> = Vec::with_capacity(BRANCH_MAX);
    for node in nodes {
        chunk.push(node);
        if chunk.len() == BRANCH_MAX {
            let taken = std::mem::take(&mut chunk);
            next.push(Arc::new(Node::Internal(Internal::from_children(taken))));
        }
    }
    flush_chunk(&mut chunk, &mut next);
    next
}

fn flush_chunk(chunk: &mut Vec<Arc<Node>>, next: &mut Vec<Arc<Node>>) {
    if chunk.is_empty() {
        return;
    }
    // Avoid an orphan tail under BRANCH_MIN by stealing from the
    // previous group if there is one and it has room.
    if chunk.len() < BRANCH_MIN
        && let Some(prev) = next.last_mut()
        && let Node::Internal(prev_i) = Arc::make_mut(prev)
        && prev_i.children.len() + chunk.len() <= BRANCH_MAX
    {
        prev_i.children.append(chunk);
        prev_i.counts = prev_i
            .children
            .iter()
            .map(|c| c.counts())
            .fold(Counts::default(), Counts::add);
        return;
    }
    let taken = std::mem::take(chunk);
    next.push(Arc::new(Node::Internal(Internal::from_children(taken))));
}

/// Chunk a UTF-8 string into leaves of <= `LEAF_TARGET` bytes,
/// splitting only on UTF-8 char boundaries.
#[must_use]
pub fn leaves_from_str(text: &str) -> Vec<Leaf> {
    if text.is_empty() {
        return vec![Leaf::from_string(String::new())];
    }
    let mut out: Vec<Leaf> = Vec::with_capacity(text.len().div_ceil(LEAF_TARGET));
    let mut start = 0usize;
    while start < text.len() {
        let target = start.saturating_add(LEAF_TARGET).min(text.len());
        let end = align_down_char_boundary(text, target);
        let end = if end == start { text.len() } else { end };
        let chunk = text.get(start..end).unwrap_or("");
        out.push(Leaf::from_string(chunk.to_owned()));
        start = end;
    }
    out
}

const fn align_down_char_boundary(text: &str, mut idx: usize) -> usize {
    if idx >= text.len() {
        return text.len();
    }
    while idx > 0 && !text.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#[path = "rope_node_tests.rs"]
mod tests;
