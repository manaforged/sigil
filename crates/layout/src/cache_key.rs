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

use ahash::AHasher;
use sigil_hal::{Dim, Size};
use sigil_kernel::{Cx, NodeId, NodeType, TextContent};
use sigil_style::StyleBinding;
use smallvec::SmallVec;

pub(crate) enum CacheDecision {
    Hit,
    Miss(LayoutCacheKey),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LayoutCacheKey {
    pub(crate) cx_instance: u64,
    pub(crate) root: NodeId,
    pub(crate) available: AvailableKey,
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
        // Reactive style bindings + reactive text content can mutate
        // without going through any kernel API that bumps
        // `Node::version` (Signal::set is a separate channel). Hash
        // their resolved value so the signature still invalidates on
        // signal write. Static/dynamic style atoms are baked into
        // `Node::version` by `attach`, so the cheap fast-path covers
        // them without paying for the AHashMap lookup here.
        if let Some(binding) = node.get_extension::<StyleBinding>() {
            let reactive = binding.reactive_slice();
            if !reactive.is_empty() {
                for atom in reactive {
                    atom.hash(&mut hasher);
                }
            }
        }
        if let NodeType::Text {
            content: content @ TextContent::Reactive(_),
        } = &node.kind
        {
            hash_text_content(content, &mut hasher);
        }
        for child in node.children().iter().rev().copied() {
            stack.push(child);
        }
    }
    hasher.finish()
}

fn hash_text_content(content: &TextContent, hasher: &mut AHasher) {
    match content {
        TextContent::Static(value) => value.hash(hasher),
        TextContent::Reactive(signal) => signal.peek().hash(hasher),
    }
}

// ε-cache: the per-atom hashing helpers were retired alongside the
// switch to version-based signatures (atoms feed into `Node::version`
// indirectly via the style binding's `attach`, so re-hashing them per
// frame is redundant).
