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
use sigil_style::{StyleAtom, StyleBinding};
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
    let mut hasher = AHasher::default();
    let mut stack = SmallVec::<[NodeId; 64]>::new();
    stack.push(root);
    while let Some(id) = stack.pop() {
        let Some(node) = cx.get(id) else {
            continue;
        };
        id.hash(&mut hasher);
        match &node.kind {
            NodeType::Container => 0_u8.hash(&mut hasher),
            NodeType::Fragment => 1_u8.hash(&mut hasher),
            NodeType::Text { content } => {
                2_u8.hash(&mut hasher);
                hash_text_content(content, &mut hasher);
            }
        }
        if let Some(binding) = node.get_extension::<StyleBinding>() {
            hash_style_atoms(binding.static_slice(), &mut hasher);
            hash_style_atoms(binding.dynamic_slice(), &mut hasher);
            hash_style_atoms(binding.reactive_slice(), &mut hasher);
        }
        for child in node.children.iter().rev().copied() {
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

fn hash_style_atoms(atoms: &[StyleAtom], hasher: &mut AHasher) {
    for atom in atoms.iter().copied().filter(is_layout_affecting_atom) {
        atom.hash(hasher);
    }
}

const fn is_layout_affecting_atom(atom: &StyleAtom) -> bool {
    // Sync point: typography atoms listed here must stay mirrored by
    // `text_style.rs::apply_text_binding`.
    matches!(
        atom,
        StyleAtom::PaddingAll(_)
            | StyleAtom::PaddingX(_)
            | StyleAtom::PaddingY(_)
            | StyleAtom::PaddingTop(_)
            | StyleAtom::PaddingRight(_)
            | StyleAtom::PaddingBottom(_)
            | StyleAtom::PaddingLeft(_)
            | StyleAtom::MarginAll(_)
            | StyleAtom::MarginX(_)
            | StyleAtom::MarginY(_)
            | StyleAtom::MarginTop(_)
            | StyleAtom::MarginRight(_)
            | StyleAtom::MarginBottom(_)
            | StyleAtom::MarginLeft(_)
            | StyleAtom::Gap(_)
            | StyleAtom::Width(_)
            | StyleAtom::Height(_)
            | StyleAtom::MinWidth(_)
            | StyleAtom::MaxWidth(_)
            | StyleAtom::Display(_)
            | StyleAtom::FlexDirection(_)
            | StyleAtom::AlignItems(_)
            | StyleAtom::JustifyContent(_)
            | StyleAtom::FlexGrow(_)
            | StyleAtom::FlexShrink(_)
            | StyleAtom::FontSize(_)
            | StyleAtom::FontWeight(_)
            | StyleAtom::LineHeight(_)
    )
}
