use std::sync::Arc;

use super::coords;

pub const LEAF_TARGET: usize = 512;
pub const LEAF_MAX: usize = 1024;
pub const BRANCH_MAX: usize = 16;
pub const BRANCH_MIN: usize = 4;

#[derive(Debug, Clone)]
pub enum Node {
    Leaf(Leaf),
    Internal(Internal),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub bytes: usize,
    pub chars: usize,
    pub lines: usize,
    pub utf16: usize,
}

impl Counts {
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.len(),
            chars: coords::count_chars(bytes),
            lines: coords::count_lines(bytes),
            utf16: coords::count_utf16(bytes),
        }
    }

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

#[derive(Debug, Clone)]
pub struct Leaf {
    pub text: String,
    pub counts: Counts,
}

impl Leaf {
    #[must_use]
    pub fn from_string(text: String) -> Self {
        let counts = Counts::of(text.as_bytes());
        Self { text, counts }
    }

    pub fn refresh(&mut self) {
        self.counts = Counts::of(self.text.as_bytes());
    }
}

#[derive(Debug, Clone)]
pub struct Internal {
    pub children: Vec<Arc<Node>>,
    pub counts: Counts,
}

impl Internal {
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
    #[must_use]
    pub fn empty() -> Arc<Self> {
        Arc::new(Self::Leaf(Leaf::from_string(String::new())))
    }

    #[must_use]
    pub const fn counts(&self) -> Counts {
        match self {
            Self::Leaf(l) => l.counts,
            Self::Internal(i) => i.counts,
        }
    }

    #[must_use]
    pub const fn len_bytes(&self) -> usize {
        self.counts().bytes
    }

    #[must_use]
    pub const fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    #[must_use]
    pub fn depth(&self) -> usize {
        match self {
            Self::Leaf(_) => 0,
            Self::Internal(i) => 1 + i.children.first().map_or(0, |c| c.depth()),
        }
    }
}

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
