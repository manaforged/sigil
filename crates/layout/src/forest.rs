//! Owned layout forest — landing in P0.1 (W2.2) per DR-041.
//!
//! Two parallel slabs:
//!   * `nodes` — `LayoutNode` records (kernel link, children, alive flag,
//!     generation tag).
//!   * `boxes` — `ComputedBox` records, scatter-written by the measure /
//!     arrange passes without aliasing `nodes`.
//!
//! `LocalId` is a generational index over the `nodes` slab. Removing a
//! node bumps its slot's generation so stale ids never alias a recycled
//! slot. The two slabs grow together — `boxes[i]` is the computed box
//! for `nodes[i]`, and the parallel pass can scatter-write `boxes`
//! while readers borrow `nodes` immutably.
//!
//! Re-exported from `engine.rs` so module call sites can write
//! `use crate::engine::{MeasureScope, NodeRef};` per the DR API sketch.
//!
//! Bodies for block / flex / parallel solvers do **not** live here —
//! they land in P0.2 / P0.3 / P0.4 against this surface unchanged.

#![allow(clippy::redundant_pub_crate)]

use std::cell::Cell;
use std::hash::{Hash as _, Hasher as _};

use ahash::AHasher;
use sigil_kernel::{NodeId, TextContent};
use sigil_style::TextStyle;
use smallvec::SmallVec;

use crate::{LayoutError, StyleData, TextMeasure};

/// Generational index into a [`LayoutForest`] slab.
///
/// `index` is the slot offset; `tag` is the slot's generation at the
/// time the id was minted. Stale ids dereference to `None` because
/// freeing a slot bumps its generation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct LocalId {
    index: u32,
    tag: u32,
}

impl LocalId {
    /// Slot offset into the forest's slab.
    pub(crate) const fn slot(self) -> u32 {
        self.index
    }

    /// Generation tag minted with the id.
    pub(crate) const fn era(self) -> u32 {
        self.tag
    }
}

/// One node in the owned layout forest.
///
/// `kernel` links back to the originating `sigil_kernel::NodeId` when
/// the node mirrors a real kernel node. `style` is the owned style
/// (added in P0.1.5) — block / flex solvers read it through
/// [`NodeRef::style`] without dereferencing external engine state.
#[derive(Clone, Debug)]
pub(crate) struct LayoutNode {
    kernel: Option<NodeId>,
    style: StyleData,
    text: Option<LayoutText>,
    children: SmallVec<[LocalId; 4]>,
    tag: u32,
    alive: bool,
}

impl LayoutNode {
    /// Kernel `NodeId` this layout node mirrors, if any.
    pub(crate) const fn kernel(&self) -> Option<NodeId> {
        self.kernel
    }

    /// Owned style for this node.
    pub(crate) const fn style(&self) -> &StyleData {
        &self.style
    }

    /// Text measurement context for text nodes.
    pub(crate) const fn text(&self) -> Option<&LayoutText> {
        self.text.as_ref()
    }
}

/// Per-node text payload consumed by layout measurement.
#[derive(Clone, Debug)]
pub(crate) struct LayoutText {
    pub(crate) content: TextContent,
    pub(crate) style: Option<TextStyle>,
    /// Wrap to the containing block instead of hugging content.
    pub(crate) wrap: bool,
    /// Single-line, truncated with an ellipsis when narrower than content.
    pub(crate) ellipsis: bool,
}

/// The two mutually-exclusive text overflow modes, carried together so the
/// sync path passes one value instead of a widening list of `bool`s.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TextFlags {
    /// Wrap to the containing block (multi-line).
    pub(crate) wrap: bool,
    /// Single-line, ellipsis-truncated when narrower than content.
    pub(crate) ellipsis: bool,
}

/// Computed geometry for a node — populated by the measure + arrange
/// passes. Stored in a slab parallel to the node slab so the parallel
/// fan-out (P0.4) can scatter-write boxes by index without aliasing
/// the node slab.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ComputedBox {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
}

/// Box-slot record — geometry + the frame tag of the most recent write.
///
/// Solvers scatter-write through `MeasureScope::write`, which stamps the
/// current frame tag into `frame`. `LayoutDrive::computed_box` checks
/// `frame == current_frame` to distinguish "reached this measure" from
/// "stale slab entry from a prior or never-measured node." Replaces the
/// per-frame `AHashSet<LocalId>` that previously tracked reached nodes
/// (the "vector δ" internal perf plan).
#[derive(Debug, Default)]
pub(crate) struct BoxCell {
    cb: Cell<ComputedBox>,
    frame: Cell<u32>,
}

impl BoxCell {
    /// Construct a slot seeded with `cb` and a frame tag of 0.
    #[must_use]
    pub(crate) const fn new(cb: ComputedBox) -> Self {
        Self {
            cb: Cell::new(cb),
            frame: Cell::new(0),
        }
    }

    /// Read the stored geometry.
    pub(crate) const fn get(&self) -> ComputedBox {
        self.cb.get()
    }

    /// Frame tag stamped on the last write.
    pub(crate) const fn frame(&self) -> u32 {
        self.frame.get()
    }

    /// Write geometry and stamp the frame tag.
    pub(crate) fn write(&self, cb: ComputedBox, frame: u32) {
        self.cb.set(cb);
        self.frame.set(frame);
    }

    /// Reset the slot (used by `free` to drop the prior occupant's data).
    pub(crate) fn reset(&self) {
        self.cb.set(ComputedBox::default());
        self.frame.set(0);
    }
}

/// Owned layout forest — the heart of the P0 engine.
///
/// Slab-backed and indexed by generational [`LocalId`]. Two parallel
/// vectors (`nodes` and `boxes`) grow together; `free` is a stack of
/// recyclable slot ids whose `tag` already reflects the post-free
/// generation, so the next `insert` mints them with the bumped tag.
#[derive(Debug)]
pub(crate) struct LayoutForest {
    nodes: Vec<LayoutNode>,
    boxes: Vec<BoxCell>,
    free: Vec<LocalId>,
    /// Monotonic frame counter — bumped by `next_frame` at the start of
    /// every measure pass; stamped onto every box slot the solvers
    /// write through `MeasureScope::write`. Starts at 1 so unwritten
    /// `BoxCell` slots (default frame 0) are always stale.
    frame: u32,
}

impl Default for LayoutForest {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutForest {
    /// Build an empty forest. The frame counter starts at 1 so the
    /// default `BoxCell::frame = 0` is always stale relative to
    /// `current_frame`; this makes `boxed_when_current` correct even
    /// without an explicit `next_frame()` before the first measure.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            boxes: Vec::new(),
            free: Vec::new(),
            frame: 1,
        }
    }

    /// Advance the frame counter and return the new tag. The driver
    /// calls this before dispatching the solver; solvers stamp every
    /// box write with this tag.
    pub(crate) const fn next_frame(&mut self) -> u32 {
        self.frame = self.frame.wrapping_add(1);
        self.frame
    }

    /// Current frame tag (most recently issued by `next_frame`).
    pub(crate) const fn current_frame(&self) -> u32 {
        self.frame
    }

    /// Live node count (alive slots only).
    pub(crate) fn count(&self) -> usize {
        self.nodes.iter().filter(|n| n.alive).count()
    }

    /// Insert a node with the given kernel link and owned style; return
    /// its generational id.
    ///
    /// Reuses a freed slot when one is available; otherwise extends
    /// both slabs by one.
    pub(crate) fn insert(&mut self, kernel: Option<NodeId>, style: StyleData) -> LocalId {
        if let Some(slot) = self.free.pop() {
            let idx = slot.index as usize;
            let node = LayoutNode {
                kernel,
                style,
                text: None,
                children: SmallVec::new(),
                tag: slot.tag,
                alive: true,
            };
            let node_slot = self
                .nodes
                .get_mut(idx)
                .expect("free-list slot points inside nodes slab");
            *node_slot = node;
            let box_slot = self
                .boxes
                .get_mut(idx)
                .expect("free-list slot points inside boxes slab");
            box_slot.reset();
            return slot;
        }
        let index = u32::try_from(self.nodes.len()).expect("forest slab exceeds u32::MAX");
        let id = LocalId { index, tag: 0 };
        self.nodes.push(LayoutNode {
            kernel,
            style,
            text: None,
            children: SmallVec::new(),
            tag: 0,
            alive: true,
        });
        self.boxes.push(BoxCell::new(ComputedBox::default()));
        id
    }

    /// Free a node. Its slot is pushed onto the free list with a bumped
    /// generation tag so future `insert` mints distinguish the new
    /// occupant from any outstanding stale id.
    pub(crate) fn free(&mut self, id: LocalId) -> Result<(), LayoutError> {
        let slot = self.slot(id)?;
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the index is in-bounds");
        let next_tag = entry.tag.wrapping_add(1);
        entry.alive = false;
        entry.children.clear();
        entry.kernel = None;
        entry.style = StyleData::default();
        entry.text = None;
        entry.tag = next_tag;
        self.free.push(LocalId {
            index: id.index,
            tag: next_tag,
        });
        Ok(())
    }

    /// Append `child` to `parent`'s child list.
    pub(crate) fn attach(&mut self, parent: LocalId, child: LocalId) -> Result<(), LayoutError> {
        let _ = self.slot(child)?;
        let slot = self.slot(parent)?;
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the parent index is in-bounds");
        entry.children.push(child);
        Ok(())
    }

    /// Replace the full child list for `parent`. Skips the write when
    /// the existing list is already byte-identical — saves a
    /// `SmallVec` clear + reallocation per unchanged sync (every
    /// frame on broadcast measures where nothing structural moved).
    pub(crate) fn set_children(
        &mut self,
        parent: LocalId,
        children: &[LocalId],
    ) -> Result<(), LayoutError> {
        for child in children {
            let _ = self.slot(*child)?;
        }
        let slot = self.slot(parent)?;
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the parent index is in-bounds");
        if entry.children.as_slice() == children {
            return Ok(());
        }
        entry.children.clear();
        entry.children.extend_from_slice(children);
        Ok(())
    }

    /// Replace the style for a live node.
    pub(crate) fn set_style(&mut self, id: LocalId, style: StyleData) -> Result<(), LayoutError> {
        let slot = self.slot(id)?;
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the index is in-bounds");
        entry.style = style;
        Ok(())
    }

    /// Replace the text measurement context for a live node.
    pub(crate) fn set_text(
        &mut self,
        id: LocalId,
        text: Option<LayoutText>,
    ) -> Result<(), LayoutError> {
        let slot = self.slot(id)?;
        let entry = self
            .nodes
            .get_mut(slot)
            .expect("slot() validated the index is in-bounds");
        entry.text = text;
        Ok(())
    }

    /// Borrow the style stored on `id`. Returns `None` for stale ids.
    /// Wraps `node` so callers don't have to go through `LayoutNode`'s
    /// crate-private `style()` accessor.
    pub(crate) fn style_for(&self, id: LocalId) -> Option<&StyleData> {
        self.node(id).map(LayoutNode::style)
    }

    /// Borrow the text payload stored on `id`. Returns `None` when the
    /// id is stale or no text is attached.
    pub(crate) fn text_for(&self, id: LocalId) -> Option<&LayoutText> {
        self.node(id).and_then(LayoutNode::text)
    }

    /// Borrow a node by id. Returns `None` for stale or freed ids.
    pub(crate) fn node(&self, id: LocalId) -> Option<&LayoutNode> {
        let idx = id.index as usize;
        let entry = self.nodes.get(idx)?;
        if entry.alive && entry.tag == id.tag {
            Some(entry)
        } else {
            None
        }
    }

    /// Computed box for `id`. Returns `None` for stale ids.
    pub(crate) fn boxed(&self, id: LocalId) -> Option<ComputedBox> {
        let idx = id.index as usize;
        let entry = self.nodes.get(idx)?;
        if entry.alive && entry.tag == id.tag {
            self.boxes.get(idx).map(BoxCell::get)
        } else {
            None
        }
    }

    /// Frame tag stamped on the box for `id`. Returns `None` for stale
    /// ids. The driver compares this against `current_frame()` to detect
    /// "reached this measure" vs "stale slab entry."
    pub(crate) fn frame_for(&self, id: LocalId) -> Option<u32> {
        let idx = id.index as usize;
        let entry = self.nodes.get(idx)?;
        if entry.alive && entry.tag == id.tag {
            self.boxes.get(idx).map(BoxCell::frame)
        } else {
            None
        }
    }

    /// Scatter-write the computed box for `id`. Returns
    /// [`LayoutError::NodeMissing`] when `id` is stale.
    ///
    /// Engine-sync callers go through this `&mut self` entry point; the
    /// parallel measure pass uses [`MeasureScope::write`] which carries
    /// only a shared borrow and writes through the cell. Both paths
    /// stamp the forest's current frame tag onto the slot.
    pub(crate) fn write(&mut self, id: LocalId, value: ComputedBox) -> Result<(), LayoutError> {
        let slot = self.slot(id)?;
        let frame = self.frame;
        let cell = self
            .boxes
            .get_mut(slot)
            .expect("slot() validated the index is in-bounds");
        cell.write(value, frame);
        Ok(())
    }

    /// Borrowed handle for a single subtree root.
    pub(crate) fn refer(&self, id: LocalId) -> Option<NodeRef<'_>> {
        self.node(id).map(|_| NodeRef { forest: self, id })
    }

    /// Translate a live id to its slab index. Returns
    /// [`LayoutError::NodeMissing`] when the id is stale, and the
    /// caller must convert to its own error type when no kernel id is
    /// available. The placeholder uses `NodeId::default()`-style
    /// rendering at the error site.
    fn slot(&self, id: LocalId) -> Result<usize, LayoutError> {
        let idx = id.index as usize;
        let entry = self
            .nodes
            .get(idx)
            .filter(|n| n.alive && n.tag == id.tag)
            .ok_or_else(|| {
                LayoutError::LayoutInvalid(format!(
                    "stale layout-forest id index={} tag={}",
                    id.index, id.tag
                ))
            })?;
        let _ = entry;
        Ok(idx)
    }
}

/// Borrowed reference to a node inside a [`LayoutForest`].
///
/// Solvers receive `NodeRef` rather than the forest directly so they
/// cannot mutate sibling subtrees mid-pass. The lifetime ties the
/// reference to a single immutable borrow of the forest.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NodeRef<'a> {
    forest: &'a LayoutForest,
    id: LocalId,
}

impl NodeRef<'_> {
    /// Identity of the referenced node.
    pub(crate) const fn id(&self) -> LocalId {
        self.id
    }

    /// Children of the referenced node in insertion order.
    pub(crate) fn children(&self) -> &[LocalId] {
        self.forest
            .node(self.id)
            .map_or(&[], |entry| entry.children.as_slice())
    }

    /// Kernel link, if any.
    pub(crate) fn kernel(&self) -> Option<NodeId> {
        self.forest.node(self.id).and_then(LayoutNode::kernel)
    }

    /// Owned style for this node. Falls back to a default style when
    /// the id is stale — solvers should not be running on stale ids,
    /// so the fallback is defensive rather than load-bearing.
    pub(crate) fn style(&self) -> &StyleData {
        self.forest
            .node(self.id)
            .map_or(&DEFAULT_STYLE, LayoutNode::style)
    }

    /// Text measurement context for this node, if it mirrors text.
    pub(crate) fn text(&self) -> Option<&LayoutText> {
        self.forest.node(self.id).and_then(LayoutNode::text)
    }
}

/// Default-style singleton for [`NodeRef::style`]'s stale-id fallback.
/// Avoids returning a temporary borrow.
static DEFAULT_STYLE: StyleData = StyleData {
    display: crate::style_data::Display::Block,
    position: crate::style_data::Position::Relative,
    overflow: crate::style_data::Size {
        width: crate::style_data::Overflow::Visible,
        height: crate::style_data::Overflow::Visible,
    },
    inset: crate::style_data::Rect {
        left: crate::style_data::LengthPercentageAuto::Auto,
        right: crate::style_data::LengthPercentageAuto::Auto,
        top: crate::style_data::LengthPercentageAuto::Auto,
        bottom: crate::style_data::LengthPercentageAuto::Auto,
    },
    flex_direction: crate::style_data::FlexDirection::Row,
    flex_wrap: crate::style_data::FlexWrap::NoWrap,
    flex_grow: 0.0,
    flex_shrink: 1.0,
    flex_basis: crate::style_data::Dimension::Auto,
    justify_content: None,
    align_items: None,
    align_content: None,
    align_self: None,
    gap: crate::style_data::Size {
        width: crate::style_data::LengthPercentage::Px(0.0),
        height: crate::style_data::LengthPercentage::Px(0.0),
    },
    margin: crate::style_data::Rect {
        left: crate::style_data::LengthPercentageAuto::Px(0.0),
        right: crate::style_data::LengthPercentageAuto::Px(0.0),
        top: crate::style_data::LengthPercentageAuto::Px(0.0),
        bottom: crate::style_data::LengthPercentageAuto::Px(0.0),
    },
    padding: crate::style_data::Rect {
        left: crate::style_data::LengthPercentage::Px(0.0),
        right: crate::style_data::LengthPercentage::Px(0.0),
        top: crate::style_data::LengthPercentage::Px(0.0),
        bottom: crate::style_data::LengthPercentage::Px(0.0),
    },
    border: crate::style_data::Rect {
        left: crate::style_data::LengthPercentage::Px(0.0),
        right: crate::style_data::LengthPercentage::Px(0.0),
        top: crate::style_data::LengthPercentage::Px(0.0),
        bottom: crate::style_data::LengthPercentage::Px(0.0),
    },
    size: crate::style_data::Size {
        width: crate::style_data::Dimension::Auto,
        height: crate::style_data::Dimension::Auto,
    },
    min_size: crate::style_data::Size {
        width: crate::style_data::Dimension::Auto,
        height: crate::style_data::Dimension::Auto,
    },
    max_size: crate::style_data::Size {
        width: crate::style_data::Dimension::Auto,
        height: crate::style_data::Dimension::Auto,
    },
    order: 0,
    box_sizing: crate::style_data::BoxSizing::ContentBox,
    grid_template_columns: Vec::new(),
    grid_template_rows: Vec::new(),
    grid_column_span: 1,
    grid_row_span: 1,
    grid_column_start: 0,
    grid_row_start: 0,
    grid_auto_flow_dense: false,
};

/// Per-axis constraint passed to a measure pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SpaceConstraint {
    /// Fixed amount of space (px).
    Definite(f32),
    /// Solver should return its minimum content size on this axis.
    MinContent,
    /// Solver should return its maximum content size on this axis.
    MaxContent,
}

/// Pair of axis constraints handed to a measure pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AvailableSpace {
    /// Inline-axis constraint.
    pub(crate) w: SpaceConstraint,
    /// Block-axis constraint.
    pub(crate) h: SpaceConstraint,
}

impl AvailableSpace {
    /// Both axes definite (px, px).
    #[must_use]
    pub(crate) const fn definite(w: f32, h: f32) -> Self {
        Self {
            w: SpaceConstraint::Definite(w),
            h: SpaceConstraint::Definite(h),
        }
    }
}

/// Borrowed scope handed to a single layout pass.
///
/// Holds an immutable view of the forest, the text measurer, and the
/// per-pass available space. The forest is shared immutably for the
/// duration of the pass so multiple solvers can read concurrently
/// (parallel measure, P0.4).
pub(crate) struct MeasureScope<'a> {
    forest: &'a LayoutForest,
    text: &'a dyn TextMeasure,
    available: AvailableSpace,
}

impl<'a> MeasureScope<'a> {
    /// Build a scope around a forest snapshot, a text measurer, and a
    /// constraint pair.
    pub(crate) const fn new(
        forest: &'a LayoutForest,
        text: &'a dyn TextMeasure,
        available: AvailableSpace,
    ) -> Self {
        Self {
            forest,
            text,
            available,
        }
    }

    /// Forest the scope is reading from.
    pub(crate) const fn forest(&self) -> &LayoutForest {
        self.forest
    }

    /// Text measurer for this pass.
    pub(crate) const fn text(&self) -> &dyn TextMeasure {
        self.text
    }

    /// Per-axis constraints for this pass.
    pub(crate) const fn available(&self) -> AvailableSpace {
        self.available
    }

    /// Scatter-write a computed box into the forest's result slab.
    ///
    /// Safe because each [`LocalId`] is unique to one node, so concurrent
    /// scatter-writes from a parallel measure pass cannot alias. The
    /// shared borrow on `LayoutForest` reflects this: nodes are read,
    /// boxes are scattered independently through the per-slot `Cell`.
    ///
    /// Stale ids (slot freed, index out of bounds) are silently ignored
    /// — same posture as the engine-sync write path's stale-id error,
    /// minus the `Result` channel which the frozen solver signature
    /// already owns for surfacing layout-level errors.
    pub(crate) fn write(&self, id: LocalId, value: ComputedBox) {
        let idx = id.index as usize;
        let Some(entry) = self.forest.nodes.get(idx) else {
            return;
        };
        if !entry.alive || entry.tag != id.tag {
            return;
        }
        if let Some(slot) = self.forest.boxes.get(idx) {
            slot.write(value, self.forest.current_frame());
        }
    }

    /// Ergonomic helper: scatter-write through a [`NodeRef`].
    pub(crate) fn write_node(&self, node: &NodeRef<'_>, value: ComputedBox) {
        self.write(node.id(), value);
    }

    /// Measure text attached to `node`, preserving any already-known
    /// axis supplied by the caller.
    pub(crate) fn measure_text(
        &self,
        node: NodeRef<'_>,
        known: crate::style_data::Size<Option<f32>>,
        wrap_to: Option<f32>,
    ) -> Option<crate::style_data::Size<f32>> {
        let text = node.text()?;
        let memo_key = text_memo_key(node.id(), known, wrap_to, text);
        if let Some(hit) = text_memo_get(memo_key) {
            return Some(hit);
        }
        // A wrapping node measures against its containing block's content
        // width (explicit width first, else the `wrap_to` the solver
        // supplies), so the paragraph engine breaks lines there. A
        // non-wrapping node measures unconstrained (max-content) — it must
        // never break at the pass's definite available (viewport) width.
        let width = if text.wrap {
            known.width.or(wrap_to).map_or_else(
                || space_constraint_to_hal_dim(self.available.w),
                sigil_hal::Dim::Fixed,
            )
        } else {
            sigil_hal::Dim::MaxContent
        };
        let space = sigil_hal::Size {
            width,
            height: space_constraint_to_hal_dim(self.available.h),
        };
        let measured = text.style.as_ref().map_or_else(
            || self.text.measure(&text.content, space),
            |style| self.text.measure_styled(&text.content, style, space),
        );
        // An ellipsizing node stays one line (measured above, unconstrained)
        // but must not force its container wider than the box: clamp the
        // reported width to the containing block so a long value fits and
        // the paint pass truncates it, while a short value still hugs.
        let natural_w = if text.ellipsis {
            known
                .width
                .or(wrap_to)
                .map_or(measured.width, |box_w| measured.width.min(box_w))
        } else {
            measured.width
        };
        let out = crate::style_data::Size {
            width: known.width.unwrap_or(natural_w),
            height: known.height.unwrap_or(measured.height),
        };
        text_memo_store(memo_key, out);
        Some(out)
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct TextMemoKey {
    index: u32,
    tag: u32,
    known_w: u32,
    known_h: u32,
    wrap: u32,
    content: u64,
}

fn text_memo_key(
    id: LocalId,
    known: crate::style_data::Size<Option<f32>>,
    wrap_to: Option<f32>,
    text: &LayoutText,
) -> TextMemoKey {
    TextMemoKey {
        index: id.slot(),
        tag: id.era(),
        known_w: known.width.map_or(u32::MAX, f32::to_bits),
        known_h: known.height.map_or(u32::MAX, f32::to_bits),
        wrap: wrap_to.map_or(u32::MAX, f32::to_bits),
        content: text_memo_content(text),
    }
}

fn text_memo_content(text: &LayoutText) -> u64 {
    let mut hasher = AHasher::default();
    match &text.content {
        TextContent::Static(value) => value.hash(&mut hasher),
        live => live.resolve().hash(&mut hasher),
    }
    text.wrap.hash(&mut hasher);
    text.ellipsis.hash(&mut hasher);
    if let Some(style) = &text.style {
        crate::text_style::text_style_signature(style).hash(&mut hasher);
    }
    hasher.finish()
}

fn text_memo_get(key: TextMemoKey) -> Option<crate::style_data::Size<f32>> {
    TEXT_MEMO.with(|cell| {
        let memo = cell.borrow();
        memo.get(&key).copied()
    })
}

fn text_memo_store(key: TextMemoKey, size: crate::style_data::Size<f32>) {
    TEXT_MEMO.with(|cell| {
        let mut memo = cell.borrow_mut();
        if memo.len() > 8_192 {
            memo.clear();
        }
        memo.insert(key, size);
    });
}

/// Drop the intra-solve text memo.
///
/// The memo is thread-local and keyed by `LocalId` + constraints +
/// content. A new forest restarts `LocalId`s, so leftover entries
/// would look like hits and skip `TextMeasure`. Call this when a new
/// drive is created.
pub(crate) fn text_memo_clear() {
    TEXT_MEMO.with(|cell| cell.borrow_mut().clear());
}

thread_local! {
    static TEXT_MEMO: std::cell::RefCell<
        ahash::AHashMap<TextMemoKey, crate::style_data::Size<f32>>,
    > = std::cell::RefCell::new(ahash::AHashMap::new());
}

const fn space_constraint_to_hal_dim(space: SpaceConstraint) -> sigil_hal::Dim {
    match space {
        SpaceConstraint::Definite(value) => sigil_hal::Dim::Fixed(value),
        SpaceConstraint::MinContent => sigil_hal::Dim::MinContent,
        SpaceConstraint::MaxContent => sigil_hal::Dim::MaxContent,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
#[path = "forest_tests.rs"]
mod tests;
