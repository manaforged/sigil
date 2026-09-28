#![allow(clippy::redundant_pub_crate)]

use std::collections::BTreeSet;
use std::hash::{Hash as _, Hasher as _};

use crate::hal::{Dim, Size};
use crate::kernel::{Cx, Node, NodeId, NodeType, TextContent};
use crate::style::{NumericAtomKind, StyleBinding};
use ahash::AHasher;
use slotmap::DefaultKey;
use smallvec::SmallVec;

use crate::layout::LayoutStyle;
use crate::layout::style_data::Dimension;

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
    pub(crate) fonts: u64,
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

pub(crate) fn has_unversioned_layout(node: &Node) -> bool {
    if node
        .get_extension::<crate::layout::ReactivePosition>()
        .is_some()
    {
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

pub(crate) fn reactive_layout_signature(cx: &Cx, ids: &BTreeSet<DefaultKey>) -> u64 {
    let mut hasher = AHasher::default();
    for &key in ids {
        let id = NodeId::from_key(key);
        let Some(node) = cx.get(id) else {
            continue;
        };
        key.hash(&mut hasher);
        hash_unversioned_layout(node, &mut hasher);
    }
    hasher.finish()
}

pub(crate) fn reactive_seed_signature(node: &Node) -> u64 {
    let mut hasher = AHasher::default();
    hash_unversioned_layout(node, &mut hasher);
    if let Some(binding) = node.get_extension::<StyleBinding>() {
        for reactive in binding.reactive_channels() {
            for atom in reactive {
                atom.hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

pub(crate) fn hash_unversioned_layout(node: &Node, hasher: &mut AHasher) -> bool {
    let hidden = node.get_extension::<StyleBinding>().is_some_and(|binding| {
        for reactive in binding.reactive_channels() {
            for atom in reactive {
                if atom.affects_layout() {
                    atom.hash(hasher);
                }
            }
        }
        for atom in binding.state_overlay_atoms() {
            if atom.affects_layout() {
                atom.hash(hasher);
            }
        }
        if let Some(overlay) = binding.animator_overlay() {
            for kind in NumericAtomKind::ALL
                .into_iter()
                .filter(|kind| kind.affects_layout())
            {
                if let Some(value) = overlay.value_for(kind) {
                    kind.hash(hasher);
                    value.to_bits().hash(hasher);
                }
            }
        }
        binding.hides()
    });
    if let Some(pos) = node.get_extension::<crate::layout::ReactivePosition>() {
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
            Dimension::Px(_) | Dimension::Percent(_) | Dimension::Fill | Dimension::Text(_)
        )
    })
}

fn hash_text_content(content: &TextContent, hasher: &mut AHasher) {
    content.resolve().hash(hasher);
}
