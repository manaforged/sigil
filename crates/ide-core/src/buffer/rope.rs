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
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: Node::empty(),
        }
    }

    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(text: &str) -> Self {
        Self {
            root: build_from_leaves(leaves_from_str(text)),
        }
    }

    pub fn from_reader<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        Ok(Self::from_str(&buf))
    }

    #[must_use]
    pub fn len_bytes(&self) -> usize {
        self.root.counts().bytes
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len_bytes() == 0
    }

    #[must_use]
    pub fn len_chars(&self) -> usize {
        self.root.counts().chars
    }

    #[must_use]
    pub fn len_lines(&self) -> usize {
        self.root.counts().lines
    }

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

    pub fn remove(&mut self, byte_range: Range<usize>) {
        if byte_range.is_empty() {
            return;
        }
        let root = Arc::make_mut(&mut self.root);
        remove_range(root, byte_range);
        self.maybe_rebalance();
    }

    #[must_use]
    pub fn slice(&self, byte_range: Range<usize>) -> RopeSlice<'_> {
        let data = collect_bytes(&self.root, byte_range);
        RopeSlice {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn line(&self, line_idx: usize) -> RopeSlice<'_> {
        let start = self.line_byte_start(line_idx);
        let end = self.line_byte_end(line_idx);
        self.slice(start..end)
    }

    pub fn write_to<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        write_node(&self.root, &mut writer)
    }

    #[must_use]
    pub fn snapshot(&self) -> Self {
        Self {
            root: Arc::clone(&self.root),
        }
    }

    #[must_use]
    pub fn same_root(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.root, &other.root)
    }

    #[must_use]
    pub fn to_string_lossy(&self) -> String {
        let mut out = String::with_capacity(self.len_bytes());
        collect_text(&self.root, &mut out);
        out
    }

    #[must_use]
    pub fn byte_to_char(&self, byte: ByteIdx) -> CharIdx {
        CharIdx(byte_to_char_in_rope(&self.root, byte.0))
    }

    #[must_use]
    pub fn char_to_byte(&self, char: CharIdx) -> ByteIdx {
        ByteIdx(char_to_byte_in_rope(&self.root, char.0))
    }

    #[must_use]
    pub fn byte_to_linecol(&self, byte: ByteIdx) -> LineCol {
        byte_to_linecol_in_rope(&self.root, byte.0)
    }

    #[must_use]
    pub fn linecol_to_byte(&self, lc: LineCol) -> ByteIdx {
        ByteIdx(linecol_to_byte_in_rope(&self.root, lc))
    }

    #[must_use]
    pub fn byte_to_utf16(&self, byte: ByteIdx) -> usize {
        byte_to_utf16_in_rope(&self.root, byte.0)
    }

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

pub struct RopeSlice<'a> {
    data: Vec<u8>,
    _marker: std::marker::PhantomData<&'a Rope>,
}

impl<'a> RopeSlice<'a> {
    #[must_use]
    pub const fn len_bytes(&self) -> usize {
        self.data.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }

    #[must_use]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }

    #[must_use]
    pub fn bytes(&'a self) -> RopeBytesIter<'a> {
        RopeBytesIter::new(&self.data)
    }

    #[must_use]
    pub fn chars(&'a self) -> RopeCharsIter<'a> {
        RopeCharsIter::new(self.as_str().unwrap_or(""))
    }

    #[must_use]
    pub fn lines(&'a self) -> RopeLinesIter<'a> {
        RopeLinesIter::new(self.as_str().unwrap_or(""))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::missing_panics_doc
)]
#[path = "rope_tests.rs"]
mod tests;
