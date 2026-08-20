//! Cache-key for [`LayoutEngine::compute`].
//!
//! Two invalidation channels feed the key:
//!   * `cx_instance` + `cx_generation` catch tree mutations routed
//!     through `Cx` (mirrors the Phase 10 / DR-023 design).
//!   * `input_signature` walks the subtree and hashes layout-affecting
//!     inputs (node kind, text content, layout-affecting style atoms).
//!     Reactive signal writes do not bump `Cx::mutation_generation`, so
//!     the content-hash is the only mechanism that catches text/style
//!     changes coming straight from `Signal::set`.

#![allow(clippy::redundant_pub_crate)]

use std::hash::{Hash as _, Hasher as _};

use ahash::{AHashSet, AHasher};
use sigil_hal::{Dim, Size};
use sigil_kernel::{Cx, Node, NodeId, NodeType, TextContent};
use sigil_style::StyleBinding;
use slotmap::DefaultKey;
use smallvec::SmallVec;

use crate::LayoutStyle;
use crate::style_data::Dimension;

#[derive(Clone, Copy, Debug)]
pub(crate) enum CacheDecision {
    Hit,
    Miss(LayoutCacheKey),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LayoutCacheKey {
    pub(crate) cx_instance: u64,
    pub(crate) root: NodeId,
    pub(crate) available: AvailableKey,
    pub(crate) scale_bits: u32,
    pub(crate) cx_generation: u64,
    pub(crate) input_signature: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AvailableKey {
    width: DimKey,
    height: DimKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DimKey {
    Fixed(u32),
    MinContent,
    MaxContent,
}

impl AvailableKey {
    pub(crate) const fn from_size(size: Size) -> Self {
        Self {
            width: DimKey::from_dim(size.width),
            height: DimKey::from_dim(size.height),
        }
    }
}

impl DimKey {
    const fn from_dim(dim: Dim) -> Self {
        match dim {
            Dim::Fixed(value) => Self::Fixed(value.to_bits()),
            Dim::MinContent => Self::MinContent,
            Dim::MaxContent => Self::MaxContent,
        }
    }
}

pub(crate) fn layout_input_signature(cx: &Cx, root: NodeId) -> u64 {
    // ε-cache: fold each node's `Node::version()` into the tree
    // signature instead of re-hashing the full style binding atoms.
    // The kernel bumps `version` on every mutation that could
    // possibly change layout output, so the version alone is a sound
    // proxy for "the deep style state at this node". For 101k-node
    // corpora this drops the signature pass from ~5 ms (AHashMap
    // probes + atom hashing per node) to ~600 µs (slotmap read +
    // version hash per node).
    //
    // Reactive text content can mutate without going through any
    // kernel mutation path — `Signal::set` does not bump
    // `Node::version` — so we still hash the materialised text
    // payload for `NodeType::Text { content: Reactive(_) }` nodes.
    // Static text is captured in `Node::version` at the time it
    // was attached.
    let mut hasher = AHasher::default();
    let mut stack = SmallVec::<[NodeId; 64]>::new();
    stack.push(root);
    while let Some(id) = stack.pop() {
        let Some(node) = cx.get(id) else {
            continue;
        };
        node.version().hash(&mut hasher);
        if hash_unversioned_layout(node, &mut hasher) {
            continue;
        }
        for child in node.children().iter().rev().copied() {
            stack.push(child);
        }
    }
    hasher.finish()
}

/// True when this node's layout input can change without bumping
/// `Node::version` — reactive position, reactive style, or wrapping /
/// indefinite non-static text.
///
/// Membership must key on channel PRESENCE (`has_reactive`), not on the
/// channel's current content: a `style_reactive` channel that starts
/// empty (`Display::None → []`) can later produce a layout-affecting
/// slice, and content-keyed membership drops the node from tracking in
/// exactly that initial state — the cache then never sees the change and
/// returns a stale hit for a now-hidden node.
pub(crate) fn has_unversioned_layout(node: &Node) -> bool {
    if node.get_extension::<crate::ReactivePosition>().is_some() {
        return true;
    }
    if node
        .get_extension::<StyleBinding>()
        .is_some_and(StyleBinding::has_reactive)
    {
        return true;
    }
    matches!(
        &node.kind,
        NodeType::Text {
            content,
            wrap,
            ..
        } if !matches!(content, TextContent::Static(_)) && (*wrap || !definite_width(node))
    )
}

/// Hash the unversioned layout inputs of `ids` in key order.
pub(crate) fn reactive_layout_signature(cx: &Cx, ids: &AHashSet<DefaultKey>) -> u64 {
    let mut keys: SmallVec<[DefaultKey; 8]> = ids.iter().copied().collect();
    keys.sort_unstable();
    let mut hasher = AHasher::default();
    for key in keys {
        let id = NodeId::from_key(key);
        let Some(node) = cx.get(id) else {
            continue;
        };
        key.hash(&mut hasher);
        hash_unversioned_layout(node, &mut hasher);
    }
    hasher.finish()
}

/// Fold unversioned layout inputs into `hasher`. Returns `true` when
/// the node is `display: none` and descendants must be skipped.
pub(crate) fn hash_unversioned_layout(node: &Node, hasher: &mut AHasher) -> bool {
    let hidden = node.get_extension::<StyleBinding>().is_some_and(|binding| {
        for reactive in binding.reactive_channels() {
            for atom in reactive {
                if atom.affects_layout() {
                    atom.hash(hasher);
                }
            }
        }
        binding.hides()
    });
    if let Some(pos) = node.get_extension::<crate::ReactivePosition>() {
        let (x, y) = pos.get();
        x.to_bits().hash(hasher);
        y.to_bits().hash(hasher);
    }
    if let NodeType::Text { content, wrap, .. } = &node.kind
        && !matches!(content, TextContent::Static(_))
        && (*wrap || !definite_width(node))
    {
        hash_text_content(content, hasher);
    }
    hidden
}

fn definite_width(node: &Node) -> bool {
    node.get_extension::<LayoutStyle>().is_some_and(|style| {
        matches!(
            style.data().size.width,
            Dimension::Px(_) | Dimension::Percent(_) | Dimension::Fill
        )
    })
}

fn hash_text_content(content: &TextContent, hasher: &mut AHasher) {
    content.resolve().hash(hasher);
}

// ε-cache: the per-atom hashing helpers were retired alongside the
// switch to version-based signatures (atoms feed into `Node::version`
// indirectly via the style binding's `attach`, so re-hashing them per
// frame is redundant).
