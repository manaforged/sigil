use std::collections::HashMap;
use std::ops::Range;

use ahash::{AHashMap, AHashSet};
use slotmap::{DefaultKey, SecondaryMap};

use crate::kernel::{Cx, FrameDelta, Node, NodeId};
use crate::layout::LayoutResult;
use crate::scene::command::SceneCommand;
use crate::scene::emit_cache;
use crate::style::{FocusRing, StyleBinding};
use crate::widgets::CanvasPaint;

use super::subtree::{Copied, Span};

#[derive(Clone, Copy, PartialEq)]
struct Seen {
    version: u32,
    layout: Option<LayoutResult>,
    volatile: (u8, u64, u64, u64, bool),
}

#[derive(Default)]
pub(crate) struct EmitMemo {
    seen: SecondaryMap<DefaultKey, Seen>,
    pub(super) spans: AHashMap<NodeId, Span>,
    selected: Vec<NodeId>,
    prev_len: Option<usize>,
    root: Option<NodeId>,
    scale: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EmitDiff {
    pub changed: Vec<(Range<usize>, Range<usize>)>,
}

impl EmitDiff {
    #[must_use]
    pub const fn unchanged(&self) -> bool {
        self.changed.is_empty()
    }

    pub(crate) fn between_scenes(old: &[SceneCommand], new: &[SceneCommand]) -> Self {
        Self::between(old, new, &[])
    }

    pub(super) fn between(old: &[SceneCommand], new: &[SceneCommand], copies: &[Copied]) -> Self {
        let mut changed = Vec::new();
        let (mut at_old, mut at_new) = (0, 0);
        for copy in copies {
            if copy.old < at_old || copy.new < at_new {
                continue;
            }
            push_gap(&mut changed, old, new, at_old..copy.old, at_new..copy.new);
            at_old = copy.old + copy.len;
            at_new = copy.new + copy.len;
        }
        push_gap(&mut changed, old, new, at_old..old.len(), at_new..new.len());
        Self { changed }
    }
}

fn push_gap(
    changed: &mut Vec<(Range<usize>, Range<usize>)>,
    old: &[SceneCommand],
    new: &[SceneCommand],
    old_range: Range<usize>,
    new_range: Range<usize>,
) {
    if old.get(old_range.clone()) != new.get(new_range.clone()) {
        changed.push((old_range, new_range));
    }
}

impl EmitMemo {
    pub(super) fn usable(&self, cx: &Cx, root: NodeId, prev: &[SceneCommand]) -> bool {
        self.prev_len == Some(prev.len())
            && self.root == Some(root)
            && self.scale == cx.scale().factor().to_bits()
    }

    pub(super) fn dirty(
        &mut self,
        cx: &Cx,
        root: NodeId,
        delta: &FrameDelta,
        selection: &HashMap<NodeId, (usize, usize)>,
    ) -> AHashSet<NodeId> {
        let mut direct: Vec<NodeId> = delta.paint.iter().chain(&delta.layout).copied().collect();
        direct.extend(self.selected.iter().copied());
        direct.extend(selection.keys().copied());
        let seen = &mut self.seen;
        cx.visit_preorder(root, |id, node| {
            let now = Seen {
                version: node.version(),
                layout: node.get_extension::<LayoutResult>().copied(),
                volatile: volatile_key(node),
            };
            if seen.insert(id.key(), now) != Some(now) {
                direct.push(id);
            }
        });
        let mut dirty = AHashSet::with_capacity(direct.len() * 4);
        for id in direct {
            let mut at = Some(id);
            while let Some(step) = at {
                if !dirty.insert(step) {
                    break;
                }
                at = cx.parent(step);
            }
        }
        dirty
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one commit records the spans, the selection, the scene length and the root"
    )]
    pub(super) fn commit(
        &mut self,
        cx: &Cx,
        root: NodeId,
        spans: AHashMap<NodeId, Span>,
        selection: &HashMap<NodeId, (usize, usize)>,
        len: usize,
    ) {
        self.spans = spans;
        self.selected = selection.keys().copied().collect();
        self.prev_len = Some(len);
        self.root = Some(root);
        self.scale = cx.scale().factor().to_bits();
    }
}

fn volatile_key(node: &Node) -> (u8, u64, u64, u64, bool) {
    let binding = node.get_extension::<StyleBinding>();
    let animated = binding.and_then(StyleBinding::animator_overlay).map_or(
        0,
        crate::style::animator_overlay::AnimatorOverlay::cache_key,
    );
    (
        emit_cache::slot_tag(node),
        animated,
        binding.map_or(0, channels_key),
        node.get_extension::<CanvasPaint>()
            .map_or(0, CanvasPaint::version),
        node.get_extension::<FocusRing>()
            .is_some_and(FocusRing::visible),
    )
}

fn channels_key(binding: &StyleBinding) -> u64 {
    let mut hasher = ahash::AHasher::default();
    for atoms in binding.reactive_channels() {
        std::ptr::hash(atoms, &mut hasher);
    }
    std::hash::Hasher::finish(&hasher)
}
