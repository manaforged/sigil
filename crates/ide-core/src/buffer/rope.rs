//! The owned rope — a persistent B-tree text buffer.
//!
//! # Persistence model
//!
//! The root is an `Arc<Node>`. Cloning the `Rope` does a shallow
//! reference-count bump: O(1). `insert` / `remove` use `Arc::make_mut`
//! to copy-on-write only the path from root to the affected leaf, so
//! snapshots before the edit are still valid (the substrate for I-T3
//! history).
//!
//! # Coordinate convention
//!
//! All public `insert` / `remove` / `slice` / `line` offsets are
//! **UTF-8 byte offsets**. The [`coords`] module provides conversions
//! to char, line/col, and UTF-16 units. Splitting on a non-boundary
//! byte position is a logic error; the functions panic in debug mode.

use std::io;
use std::ops::Range;
use std::sync::Arc;

use super::{
    coords::{
        ByteIdx, CharIdx, LineCol, byte_to_char_in_leaf, byte_to_utf16_in_leaf,
        char_to_byte_in_leaf, line_start_in_leaf, lines_before_byte_in_leaf, utf16_to_byte_in_leaf,
    },
    rope_iter::{RopeBytesIter, RopeCharsIter, RopeLinesIter},
    rope_node::{
        Counts, Internal, LEAF_MAX, LEAF_TARGET, Leaf, Node, build_from_leaves, leaves_from_str,
    },
};

/// An owned, persistent UTF-8 rope buffer.
///
/// `clone()` is O(1) (bumps the root `Arc`). Edits copy-on-write only
/// the affected path from root to leaf.
#[derive(Debug, Clone)]
pub struct Rope {
    root: Arc<Node>,
}

impl Default for Rope {
    fn default() -> Self {
        Self::new()
    }
}

impl Rope {
    /// Create an empty rope.
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: Node::empty(),
        }
    }

    /// Build a rope from a UTF-8 string slice.
    ///
    /// The method is named `from_str` to match the brief API surface
    /// (W3.5a) and standard Rust `From*` naming convention. The
    /// `should_implement_trait` lint is suppressed here because this
    /// method is infallible — `std::str::FromStr` would require returning
    /// a `Result` which adds noise at every call site for a can't-fail op.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(text: &str) -> Self {
        Self {
            root: build_from_leaves(leaves_from_str(text)),
        }
    }

    /// Read a UTF-8 stream into a rope.
    ///
    /// Reads the entire stream into a `String` first, then slices into
    /// leaves. A streaming version can be added later if memory is a concern.
    pub fn from_reader<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        Ok(Self::from_str(&buf))
    }

    /// Number of UTF-8 bytes in the buffer.
    #[must_use]
    pub fn len_bytes(&self) -> usize {
        self.root.counts().bytes
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len_bytes() == 0
    }

    /// Number of Unicode scalar values in the buffer.
    #[must_use]
    pub fn len_chars(&self) -> usize {
        self.root.counts().chars
    }

    /// Number of `\n` bytes (logical line count is this + 1 unless the
    /// last byte is `\n`).
    #[must_use]
    pub fn len_lines(&self) -> usize {
        self.root.counts().lines
    }

    /// Insert `text` at `byte_pos`.
    ///
    /// `byte_pos` must be a UTF-8 char boundary in \[0, `len_bytes`\].
    pub fn insert(&mut self, byte_pos: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let root = Arc::make_mut(&mut self.root);
        if let Some(new_root) = insert_at(root, byte_pos, text) {
            self.root = new_root;
        }
        self.maybe_rebalance();
    }

    /// Remove bytes in `byte_range`.
    ///
    /// Both endpoints must be UTF-8 char boundaries in \[0, `len_bytes`\].
    pub fn remove(&mut self, byte_range: Range<usize>) {
        if byte_range.is_empty() {
            return;
        }
        let root = Arc::make_mut(&mut self.root);
        remove_range(root, byte_range);
        self.maybe_rebalance();
    }

    /// Return a slice view over `byte_range`.
    ///
    /// Collects the byte range into a `Vec<u8>` for simple access;
    /// zero-copy tree-walk iteration is a follow-up.
    #[must_use]
    pub fn slice(&self, byte_range: Range<usize>) -> RopeSlice<'_> {
        let data = collect_bytes(&self.root, byte_range);
        RopeSlice {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Return a slice view of the `line_idx`-th line (0-based).
    ///
    /// Includes the trailing `\n` if present. Returns an empty slice
    /// for `line_idx` beyond the last line.
    #[must_use]
    pub fn line(&self, line_idx: usize) -> RopeSlice<'_> {
        let start = self.line_byte_start(line_idx);
        let end = self.line_byte_end(line_idx);
        self.slice(start..end)
    }

    /// Write the entire rope to `writer`.
    pub fn write_to<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        write_node(&self.root, &mut writer)
    }

    /// Return a cheap O(1) snapshot (clones the root `Arc`).
    ///
    /// The snapshot is structurally shared with `self` until either is
    /// mutated, at which point only the modified path is copied.
    #[must_use]
    pub fn snapshot(&self) -> Self {
        Self {
            root: Arc::clone(&self.root),
        }
    }

    /// Whether `self` and `other` share the same root `Arc`.
    ///
    /// O(1). Returns `true` when one was produced from the other via
    /// `snapshot()` (or `clone()`) and neither has been mutated since.
    /// Used by [`super::history::History`] to dedup consecutive
    /// checkpoints — pushing two pointer-equal snapshots is pure
    /// memory waste, and silently turns one logical undo into two
    /// physical undos.
    #[must_use]
    pub fn same_root(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.root, &other.root)
    }

    /// Collect the full text into a `String`.
    #[must_use]
    pub fn to_string_lossy(&self) -> String {
        let mut out = String::with_capacity(self.len_bytes());
        collect_text(&self.root, &mut out);
        out
    }

    /// Convert a UTF-8 byte index to a Unicode char index.
    #[must_use]
    pub fn byte_to_char(&self, byte: ByteIdx) -> CharIdx {
        CharIdx(byte_to_char_in_rope(&self.root, byte.0))
    }

    /// Convert a Unicode char index to a UTF-8 byte index.
    #[must_use]
    pub fn char_to_byte(&self, char: CharIdx) -> ByteIdx {
        ByteIdx(char_to_byte_in_rope(&self.root, char.0))
    }

    /// Convert a byte offset to a line + column.
    #[must_use]
    pub fn byte_to_linecol(&self, byte: ByteIdx) -> LineCol {
        byte_to_linecol_in_rope(&self.root, byte.0)
    }

    /// Convert a line + column to a byte offset.
    #[must_use]
    pub fn linecol_to_byte(&self, lc: LineCol) -> ByteIdx {
        ByteIdx(linecol_to_byte_in_rope(&self.root, lc))
    }

    /// Convert a byte offset to a UTF-16 code-unit offset (for LSP).
    #[must_use]
    pub fn byte_to_utf16(&self, byte: ByteIdx) -> usize {
        byte_to_utf16_in_rope(&self.root, byte.0)
    }

    /// Convert a UTF-16 code-unit offset to a byte offset (for LSP).
    #[must_use]
    pub fn utf16_to_byte(&self, utf16: usize) -> ByteIdx {
        ByteIdx(utf16_to_byte_in_rope(&self.root, utf16))
    }

    fn line_byte_start(&self, line_idx: usize) -> usize {
        linecol_to_byte_in_rope(
            &self.root,
            LineCol {
                line: line_idx,
                col: 0,
            },
        )
    }

    fn line_byte_end(&self, line_idx: usize) -> usize {
        let start = self.line_byte_start(line_idx);
        if self.len_bytes().saturating_sub(start) == 0 {
            return start;
        }
        let slice = collect_bytes(&self.root, start..self.len_bytes());
        slice
            .iter()
            .position(|&b| b == b'\n')
            .map_or_else(|| self.len_bytes(), |nl| start + nl + 1)
    }

    /// After an insert/remove, if the root is an `Internal` with one
    /// child, unwrap it so the tree stays balanced.
    fn maybe_rebalance(&mut self) {
        loop {
            match self.root.as_ref() {
                Node::Internal(i) if i.children.len() == 1 => {
                    let child = Arc::clone(i.children.first().expect("checked len == 1"));
                    self.root = child;
                }
                _ => break,
            }
        }
    }
}

// ---- Insert ------------------------------------------------------------

/// Insert `text` into `node` at `byte_pos`, returning a new sibling
/// `Arc` if the node split (the parent absorbs it).
fn insert_at(node: &mut Node, byte_pos: usize, text: &str) -> Option<Arc<Node>> {
    match node {
        Node::Leaf(leaf) => insert_at_leaf(leaf, byte_pos, text),
        Node::Internal(internal) => insert_at_internal(internal, byte_pos, text),
    }
}

fn insert_at_leaf(leaf: &mut Leaf, byte_pos: usize, text: &str) -> Option<Arc<Node>> {
    leaf.text.insert_str(byte_pos, text);
    leaf.refresh();
    if leaf.text.len() > LEAF_MAX {
        Some(split_leaf(leaf))
    } else {
        None
    }
}

fn split_leaf(leaf: &mut Leaf) -> Arc<Node> {
    let mid = align_split_point(&leaf.text, LEAF_TARGET);
    let right_text = leaf.text.split_off(mid);
    leaf.refresh();
    Arc::new(Node::Leaf(Leaf::from_string(right_text)))
}

fn align_split_point(text: &str, approx: usize) -> usize {
    let mut idx = approx.min(text.len());
    while idx > 0 && !text.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn insert_at_internal(
    internal: &mut Internal,
    mut byte_pos: usize,
    text: &str,
) -> Option<Arc<Node>> {
    let (child_idx, local_pos) = find_child(internal, byte_pos);
    byte_pos = local_pos;

    let child_arc = internal.children.get_mut(child_idx)?;
    let child = Arc::make_mut(child_arc);
    let new_sibling = insert_at(child, byte_pos, text);

    if let Some(sib) = new_sibling {
        internal.children.insert(child_idx + 1, sib);
    }
    internal.counts = internal
        .children
        .iter()
        .map(|c| c.counts())
        .fold(Counts::default(), Counts::add);

    if internal.children.len() > super::rope_node::BRANCH_MAX {
        Some(split_internal(internal))
    } else {
        None
    }
}

fn split_internal(internal: &mut Internal) -> Arc<Node> {
    let mid = internal.children.len() / 2;
    let right_children = internal.children.split_off(mid);
    internal.counts = internal
        .children
        .iter()
        .map(|c| c.counts())
        .fold(Counts::default(), Counts::add);
    Arc::new(Node::Internal(Internal::from_children(right_children)))
}

fn find_child(internal: &Internal, byte_pos: usize) -> (usize, usize) {
    let mut remaining = byte_pos;
    for (i, child) in internal.children.iter().enumerate() {
        let child_bytes = child.counts().bytes;
        let is_last = i + 1 == internal.children.len();
        if remaining <= child_bytes || is_last {
            return (i, remaining);
        }
        remaining -= child_bytes;
    }
    (0, 0)
}

// ---- Remove ------------------------------------------------------------

fn remove_range(node: &mut Node, range: Range<usize>) {
    match node {
        Node::Leaf(leaf) => remove_from_leaf(leaf, range),
        Node::Internal(internal) => remove_from_internal(internal, range),
    }
}

fn remove_from_leaf(leaf: &mut Leaf, range: Range<usize>) {
    let end = range.end.min(leaf.text.len());
    let start = range.start.min(end);
    leaf.text.replace_range(start..end, "");
    leaf.refresh();
}

fn remove_from_internal(internal: &mut Internal, range: Range<usize>) {
    let mut offset = 0usize;
    for child in &mut internal.children {
        let child_bytes = child.counts().bytes;
        let child_range = offset..offset + child_bytes;
        let overlap = range_overlap(&child_range, &range);
        if !overlap.is_empty() {
            let local = (overlap.start - offset)..(overlap.end - offset);
            remove_range(Arc::make_mut(child), local);
        }
        offset += child_bytes;
        if offset >= range.end {
            break;
        }
    }
    internal.children.retain(|c| c.counts().bytes > 0);
    if internal.children.is_empty() {
        internal.children.push(Node::empty());
    }
    internal.counts = internal
        .children
        .iter()
        .map(|c| c.counts())
        .fold(Counts::default(), Counts::add);
}

fn range_overlap(a: &Range<usize>, b: &Range<usize>) -> Range<usize> {
    let start = a.start.max(b.start);
    let end = a.end.min(b.end);
    start..end.max(start)
}

// ---- Collect bytes / text ----------------------------------------------

fn collect_bytes(node: &Node, range: Range<usize>) -> Vec<u8> {
    let mut buf = Vec::with_capacity(range.len());
    collect_bytes_into(node, range, 0, &mut buf);
    buf
}

fn collect_bytes_into(node: &Node, range: Range<usize>, node_start: usize, out: &mut Vec<u8>) {
    match node {
        Node::Leaf(leaf) => {
            let node_end = node_start + leaf.counts.bytes;
            let lo = range.start.max(node_start).saturating_sub(node_start);
            let hi = range.end.min(node_end).saturating_sub(node_start);
            if lo < hi
                && let Some(slice) = leaf.text.as_bytes().get(lo..hi)
            {
                out.extend_from_slice(slice);
            }
        }
        Node::Internal(internal) => {
            let mut offset = node_start;
            for child in &internal.children {
                let child_bytes = child.counts().bytes;
                let child_end = offset + child_bytes;
                if offset >= range.end {
                    break;
                }
                if child_end > range.start {
                    collect_bytes_into(child, range.clone(), offset, out);
                }
                offset = child_end;
            }
        }
    }
}

fn collect_text(node: &Node, out: &mut String) {
    match node {
        Node::Leaf(leaf) => out.push_str(&leaf.text),
        Node::Internal(i) => {
            for child in &i.children {
                collect_text(child, out);
            }
        }
    }
}

fn write_node<W: io::Write>(node: &Node, w: &mut W) -> io::Result<()> {
    match node {
        Node::Leaf(leaf) => w.write_all(leaf.text.as_bytes()),
        Node::Internal(i) => {
            for child in &i.children {
                write_node(child, w)?;
            }
            Ok(())
        }
    }
}

// ---- Coordinate conversion helpers -------------------------------------

fn byte_to_char_in_rope(node: &Node, byte: usize) -> usize {
    match node {
        Node::Leaf(leaf) => byte_to_char_in_leaf(leaf.text.as_bytes(), byte.min(leaf.counts.bytes)),
        Node::Internal(i) => {
            let (child, remaining, chars_before) = descend_by_bytes(i, byte);
            chars_before + byte_to_char_in_rope(child, remaining)
        }
    }
}

fn char_to_byte_in_rope(node: &Node, char_idx: usize) -> usize {
    match node {
        Node::Leaf(leaf) => char_to_byte_in_leaf(leaf.text.as_bytes(), char_idx),
        Node::Internal(i) => {
            let (child, remaining, bytes_before) = descend_by_chars(i, char_idx);
            bytes_before + char_to_byte_in_rope(child, remaining)
        }
    }
}

fn byte_to_linecol_in_rope(node: &Node, byte: usize) -> LineCol {
    match node {
        Node::Leaf(leaf) => {
            let b = byte.min(leaf.counts.bytes);
            let line = lines_before_byte_in_leaf(leaf.text.as_bytes(), b);
            let col = line_col_byte(leaf.text.as_bytes(), b);
            LineCol { line, col }
        }
        Node::Internal(i) => {
            let (child, remaining, lines_before, bytes_before_line_start) =
                descend_by_bytes_linecol(i, byte);
            let lc = byte_to_linecol_in_rope(child, remaining);
            LineCol {
                line: lines_before + lc.line,
                col: if lc.line == 0 {
                    bytes_before_line_start + lc.col
                } else {
                    lc.col
                },
            }
        }
    }
}

fn line_col_byte(bytes: &[u8], byte: usize) -> usize {
    let prefix = bytes.get(..byte).unwrap_or(&[]);
    prefix
        .iter()
        .rev()
        .position(|&b| b == b'\n')
        .unwrap_or(byte)
}

fn linecol_to_byte_in_rope(node: &Node, lc: LineCol) -> usize {
    match node {
        Node::Leaf(leaf) => {
            let line_start = line_start_in_leaf(leaf.text.as_bytes(), lc.line);
            (line_start + lc.col).min(leaf.counts.bytes)
        }
        Node::Internal(i) => {
            let (child, remaining_line, remaining_col, bytes_before) = descend_by_linecol(i, lc);
            bytes_before
                + linecol_to_byte_in_rope(
                    child,
                    LineCol {
                        line: remaining_line,
                        col: remaining_col,
                    },
                )
        }
    }
}

fn byte_to_utf16_in_rope(node: &Node, byte: usize) -> usize {
    match node {
        Node::Leaf(leaf) => {
            byte_to_utf16_in_leaf(leaf.text.as_bytes(), byte.min(leaf.counts.bytes))
        }
        Node::Internal(i) => {
            let (child, remaining, utf16_before) = descend_by_bytes_utf16(i, byte);
            utf16_before + byte_to_utf16_in_rope(child, remaining)
        }
    }
}

fn utf16_to_byte_in_rope(node: &Node, utf16: usize) -> usize {
    match node {
        Node::Leaf(leaf) => utf16_to_byte_in_leaf(leaf.text.as_bytes(), utf16),
        Node::Internal(i) => {
            let (child, remaining, bytes_before) = descend_by_utf16(i, utf16);
            bytes_before + utf16_to_byte_in_rope(child, remaining)
        }
    }
}

// ---- Descent helpers ---------------------------------------------------

/// Walk children accumulating bytes; return (`child`, `remaining_bytes`,
/// `chars_accumulated_before_child`).
fn descend_by_bytes(i: &Internal, byte: usize) -> (&Node, usize, usize) {
    let mut rem = byte;
    let mut chars_acc = 0usize;
    let last = i.children.last();
    for child in &i.children {
        let cb = child.counts().bytes;
        let cc = child.counts().chars;
        let is_last = last.is_some_and(|l| Arc::ptr_eq(l, child));
        if rem <= cb || is_last {
            return (child.as_ref(), rem, chars_acc);
        }
        rem -= cb;
        chars_acc += cc;
    }
    (
        i.children
            .last()
            .map_or_else(|| unreachable!(), Arc::as_ref),
        0,
        chars_acc,
    )
}

fn descend_by_chars(i: &Internal, char_idx: usize) -> (&Node, usize, usize) {
    let mut rem = char_idx;
    let mut bytes_acc = 0usize;
    let last = i.children.last();
    for child in &i.children {
        let cb = child.counts().bytes;
        let cc = child.counts().chars;
        let is_last = last.is_some_and(|l| Arc::ptr_eq(l, child));
        if rem <= cc || is_last {
            return (child.as_ref(), rem, bytes_acc);
        }
        rem -= cc;
        bytes_acc += cb;
    }
    (
        i.children
            .last()
            .map_or_else(|| unreachable!(), Arc::as_ref),
        0,
        bytes_acc,
    )
}

fn descend_by_bytes_utf16(i: &Internal, byte: usize) -> (&Node, usize, usize) {
    let mut rem = byte;
    let mut utf16_acc = 0usize;
    let last = i.children.last();
    for child in &i.children {
        let cb = child.counts().bytes;
        let cu = child.counts().utf16;
        let is_last = last.is_some_and(|l| Arc::ptr_eq(l, child));
        if rem <= cb || is_last {
            return (child.as_ref(), rem, utf16_acc);
        }
        rem -= cb;
        utf16_acc += cu;
    }
    (
        i.children
            .last()
            .map_or_else(|| unreachable!(), Arc::as_ref),
        0,
        utf16_acc,
    )
}

fn descend_by_utf16(i: &Internal, utf16: usize) -> (&Node, usize, usize) {
    let mut rem = utf16;
    let mut bytes_acc = 0usize;
    let last = i.children.last();
    for child in &i.children {
        let cb = child.counts().bytes;
        let cu = child.counts().utf16;
        let is_last = last.is_some_and(|l| Arc::ptr_eq(l, child));
        if rem <= cu || is_last {
            return (child.as_ref(), rem, bytes_acc);
        }
        rem -= cu;
        bytes_acc += cb;
    }
    (
        i.children
            .last()
            .map_or_else(|| unreachable!(), Arc::as_ref),
        0,
        bytes_acc,
    )
}

fn descend_by_bytes_linecol(i: &Internal, byte: usize) -> (&Node, usize, usize, usize) {
    let mut rem = byte;
    let mut lines_acc = 0usize;
    let mut bytes_before_line = 0usize;
    let last = i.children.last();
    for child in &i.children {
        let cb = child.counts().bytes;
        let cl = child.counts().lines;
        let is_last = last.is_some_and(|l| Arc::ptr_eq(l, child));
        if rem <= cb || is_last {
            return (child.as_ref(), rem, lines_acc, bytes_before_line);
        }
        rem -= cb;
        lines_acc += cl;
        bytes_before_line = 0;
    }
    (
        i.children
            .last()
            .map_or_else(|| unreachable!(), Arc::as_ref),
        0,
        lines_acc,
        bytes_before_line,
    )
}

fn descend_by_linecol(i: &Internal, lc: LineCol) -> (&Node, usize, usize, usize) {
    let mut rem_line = lc.line;
    let mut bytes_acc = 0usize;
    let last = i.children.last();
    for child in &i.children {
        let cb = child.counts().bytes;
        let cl = child.counts().lines;
        let is_last = last.is_some_and(|l| Arc::ptr_eq(l, child));
        if rem_line <= cl || is_last {
            return (child.as_ref(), rem_line, lc.col, bytes_acc);
        }
        rem_line -= cl;
        bytes_acc += cb;
    }
    (
        i.children
            .last()
            .map_or_else(|| unreachable!(), Arc::as_ref),
        rem_line,
        lc.col,
        bytes_acc,
    )
}

// ---- RopeSlice ---------------------------------------------------------

/// A borrowed view of a byte range in a [`Rope`].
///
/// Backed by a collected `Vec<u8>` for simplicity. Zero-copy tree
/// iteration is a follow-up.
pub struct RopeSlice<'a> {
    /// Collected bytes for this slice.
    data: Vec<u8>,
    _marker: std::marker::PhantomData<&'a Rope>,
}

impl<'a> RopeSlice<'a> {
    /// Length in bytes.
    #[must_use]
    pub const fn len_bytes(&self) -> usize {
        self.data.len()
    }

    /// Whether the slice is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Return as a `&str` if the data is valid UTF-8 (it always should
    /// be; returns `None` only if a bug caused a bad slice boundary).
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }

    /// Collect into a `String`.
    ///
    /// Named `to_string` to match the API surface in W3.5a. The inherent
    /// `to_string` lint is suppressed: we intentionally do NOT implement
    /// `Display` on `RopeSlice` (it would require infallible UTF-8, which
    /// we can't guarantee without the `as_str()` guard), so the standard
    /// `ToString` derivation route is blocked.
    #[must_use]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }

    /// Iterate bytes.
    #[must_use]
    pub fn bytes(&'a self) -> RopeBytesIter<'a> {
        RopeBytesIter::new(&self.data)
    }

    /// Iterate Unicode scalar values.
    #[must_use]
    pub fn chars(&'a self) -> RopeCharsIter<'a> {
        RopeCharsIter::new(self.as_str().unwrap_or(""))
    }

    /// Iterate lines (each line includes its trailing `\n` if present).
    #[must_use]
    pub fn lines(&'a self) -> RopeLinesIter<'a> {
        RopeLinesIter::new(self.as_str().unwrap_or(""))
    }
}

// ---- Smoke tests -------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::missing_panics_doc
)]
mod tests {
    use super::*;

    #[test]
    fn empty_rope_construction() {
        let r = Rope::new();
        assert_eq!(r.len_bytes(), 0);
        assert_eq!(r.len_chars(), 0);
        assert_eq!(r.len_lines(), 0);
        assert!(r.is_empty());
    }

    #[test]
    fn from_str_counts() {
        let r = Rope::from_str("hello\nworld\n");
        assert_eq!(r.len_bytes(), 12);
        assert_eq!(r.len_chars(), 12);
        assert_eq!(r.len_lines(), 2);
    }

    #[test]
    fn single_byte_insert_remove() {
        let mut r = Rope::new();
        r.insert(0, "a");
        assert_eq!(r.len_bytes(), 1);
        assert_eq!(r.to_string_lossy(), "a");
        r.remove(0..1);
        assert_eq!(r.len_bytes(), 0);
        assert!(r.is_empty());
    }

    #[test]
    fn insert_at_boundaries() {
        let mut r = Rope::from_str("world");
        r.insert(0, "hello ");
        assert_eq!(r.to_string_lossy(), "hello world");
        r.insert(r.len_bytes(), "!");
        assert_eq!(r.to_string_lossy(), "hello world!");
        r.insert(6, "beautiful ");
        assert_eq!(r.to_string_lossy(), "hello beautiful world!");
    }

    #[test]
    fn utf8_boundary_insert() {
        // "café" — 'é' is 2 bytes (U+00E9).
        let mut r = Rope::from_str("caf");
        r.insert(3, "é");
        assert_eq!(r.to_string_lossy(), "café");
        assert_eq!(r.len_chars(), 4);
        assert_eq!(r.len_bytes(), 5);
    }

    #[test]
    fn cross_leaf_insert() {
        // Build a rope bigger than one leaf, then insert in the middle.
        let base: String = "a".repeat(LEAF_TARGET + 10);
        let mut r = Rope::from_str(&base);
        let mid = LEAF_TARGET / 2;
        r.insert(mid, "X");
        let s = r.to_string_lossy();
        assert_eq!(s.len(), base.len() + 1);
        assert_eq!(s.as_bytes().get(mid), Some(&b'X'));
    }

    #[test]
    fn cross_leaf_remove() {
        let base: String = "abcdefgh".repeat(200); // 1600 bytes, 3+ leaves
        let mut r = Rope::from_str(&base);
        r.remove(500..510);
        let s = r.to_string_lossy();
        assert_eq!(s.len(), base.len() - 10);
        // Verify the content around the gap is correct.
        let mut expected = base;
        expected.replace_range(500..510, "");
        assert_eq!(s, expected);
    }

    #[test]
    fn to_string_roundtrip_large() {
        let source: String = "hello world\n".repeat(1000);
        let r = Rope::from_str(&source);
        assert_eq!(r.to_string_lossy(), source);
    }

    #[test]
    fn slice_correctness() {
        let r = Rope::from_str("hello world");
        let sl = r.slice(6..11);
        assert_eq!(sl.to_string(), "world");
    }

    #[test]
    fn line_access() {
        let r = Rope::from_str("foo\nbar\nbaz");
        assert_eq!(r.line(0).to_string(), "foo\n");
        assert_eq!(r.line(1).to_string(), "bar\n");
        assert_eq!(r.line(2).to_string(), "baz");
    }

    #[test]
    fn snapshot_is_independent() {
        let r = Rope::from_str("hello");
        let snap = r.snapshot();
        let mut r2 = r;
        r2.insert(5, " world");
        assert_eq!(snap.to_string_lossy(), "hello");
        assert_eq!(r2.to_string_lossy(), "hello world");
    }

    #[test]
    fn byte_to_char_conversion() {
        let r = Rope::from_str("🦀abc");
        // '🦀' is 4 bytes; byte 4 should be char index 1.
        assert_eq!(r.byte_to_char(ByteIdx(4)), CharIdx(1));
        assert_eq!(r.char_to_byte(CharIdx(1)), ByteIdx(4));
    }

    #[test]
    fn byte_to_linecol_conversion() {
        let r = Rope::from_str("hello\nworld");
        let lc = r.byte_to_linecol(ByteIdx(7));
        assert_eq!(lc.line, 1);
        assert_eq!(lc.col, 1);
    }

    #[test]
    fn write_to_roundtrip() {
        let source = "hello world\n";
        let r = Rope::from_str(source);
        let mut out: Vec<u8> = Vec::new();
        r.write_to(&mut out).unwrap();
        assert_eq!(out, source.as_bytes());
    }

    #[test]
    fn large_insert() {
        let big = "x".repeat(1_024 * 1_024); // 1 MB
        let r = Rope::from_str(&big);
        assert_eq!(r.len_bytes(), big.len());
        assert_eq!(r.to_string_lossy(), big);
    }
}
