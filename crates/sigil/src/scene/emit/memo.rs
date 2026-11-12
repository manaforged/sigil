use std::collections::HashMap;
#[cfg(terminal_host)]
use std::ops::Range;

use ahash::AHashSet;
use slotmap::{DefaultKey, SecondaryMap};
use smallvec::SmallVec;

use crate::geometry::Rect;
use crate::kernel::{Cx, FrameDelta, Node, NodeId};
use crate::layout::LayoutResult;
use crate::scene::canvas::CanvasPaint;
use crate::scene::command::{CommandScene, SceneCommand};
use crate::scene::emit_cache;
use crate::style::{FocusRing, StyleBinding};

#[cfg(terminal_host)]
use super::subtree::{Copied, EmitDiff};
use super::subtree::{SpanMap, SpanTrack, rect_bits};

#[derive(Clone, Copy, PartialEq)]
struct Seen {
    version: u32,
    layout: Option<LayoutResult>,
    volatile: (u8, u64, u64, u64, bool),
}

#[derive(Default)]
pub(crate) struct EmitMemo {
    seen: Option<SecondaryMap<DefaultKey, Seen>>,
    pub(super) spans: Option<SpanMap>,
    pub(super) scratch: Option<SpanMap>,
    selected: HashMap<NodeId, (usize, usize)>,
    watched: Vec<NodeId>,
    prev_len: Option<usize>,
    root: Option<NodeId>,
    scale: u32,
    clip: Option<[u32; 4]>,
    theme_epoch: u64,
}

#[derive(Default)]
pub(crate) struct CleanScene {
    watched: Vec<(NodeId, Seen)>,
    selected: HashMap<NodeId, (usize, usize)>,
    scene: Option<CommandScene>,
    probed: bool,
    root: Option<NodeId>,
    structure_gen: u64,
    paint_epoch: u64,
    layout_gen: u64,
    scale: u32,
    clip: Option<[u32; 4]>,
    theme_epoch: u64,
}

#[cfg(terminal_host)]
impl EmitDiff {
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

#[cfg(terminal_host)]
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

impl CleanScene {
    pub(crate) fn emit(
        &mut self,
        cx: &Cx,
        (root, clip): (NodeId, Option<Rect>),
        layout_gen: u64,
    ) -> CommandScene {
        let paint_epoch = cx.paint_epoch();
        if self.root != Some(root)
            || self.paint_epoch != paint_epoch
            || self.layout_gen != layout_gen
        {
            self.probed = false;
            self.scene = None;
            self.root = Some(root);
            self.paint_epoch = paint_epoch;
            self.layout_gen = layout_gen;
            return super::emit_scene(cx, root, clip);
        }
        self.emit_steady(cx, root, clip)
    }

    fn emit_steady(&mut self, cx: &Cx, root: NodeId, clip: Option<Rect>) -> CommandScene {
        let selection = crate::events::selection::ranges(cx);
        if self.probe_matches(cx, clip, &selection) {
            if let Some(scene) = &self.scene {
                return scene.clone();
            }
            let scene = super::run(cx, root, clip, &selection, &mut SpanTrack::off());
            self.scene = Some(scene.clone());
            return scene;
        }
        self.refresh_probe(cx, root, clip, &selection);
        super::run(cx, root, clip, &selection, &mut SpanTrack::off())
    }

    fn probe_matches(
        &self,
        cx: &Cx,
        clip: Option<Rect>,
        selection: &HashMap<NodeId, (usize, usize)>,
    ) -> bool {
        self.probed
            && self.structure_gen == cx.mutation_generation()
            && self.scale == cx.scale().to_bits()
            && self.clip == clip.map(rect_bits)
            && self.theme_epoch == emit_cache::theme_epoch()
            && self.selected == *selection
            && self
                .watched
                .iter()
                .all(|&(id, prev)| cx.get(id).map(seen_of) == Some(prev))
    }

    fn refresh_probe(
        &mut self,
        cx: &Cx,
        root: NodeId,
        clip: Option<Rect>,
        selection: &HashMap<NodeId, (usize, usize)>,
    ) {
        let structure_gen = cx.mutation_generation();
        if self.probed && self.structure_gen == structure_gen {
            for (id, prev) in &mut self.watched {
                if let Some(node) = cx.get(*id) {
                    *prev = seen_of(node);
                }
            }
        } else {
            self.watched.clear();
            walk_tree(cx, root, |id, node| {
                if watches(node) {
                    self.watched.push((id, seen_of(node)));
                }
            });
        }
        self.structure_gen = structure_gen;
        self.scale = cx.scale().to_bits();
        self.clip = clip.map(rect_bits);
        self.theme_epoch = emit_cache::theme_epoch();
        if self.selected != *selection {
            self.selected.clone_from(selection);
        }
        self.probed = true;
        self.scene = None;
    }
}

impl EmitMemo {
    pub(super) fn usable(
        &self,
        cx: &Cx,
        (root, clip): (NodeId, Option<Rect>),
        prev: &[SceneCommand],
    ) -> bool {
        self.prev_len == Some(prev.len())
            && self.root == Some(root)
            && self.scale == cx.scale().to_bits()
            && self.clip == clip.map(rect_bits)
            && self.theme_epoch == emit_cache::theme_epoch()
    }

    pub(crate) fn clean(
        &self,
        cx: &Cx,
        (root, clip): (NodeId, Option<Rect>),
        (delta, prev_len): (&FrameDelta, usize),
        selection: &HashMap<NodeId, (usize, usize)>,
    ) -> bool {
        delta.is_empty()
            && self.prev_len == Some(prev_len)
            && self.root == Some(root)
            && self.scale == cx.scale().to_bits()
            && self.clip == clip.map(rect_bits)
            && self.theme_epoch == emit_cache::theme_epoch()
            && self.selected == *selection
            && self.watched.iter().all(|id| {
                cx.get(*id).map(seen_of)
                    == self
                        .seen
                        .as_ref()
                        .and_then(|seen| seen.get(id.key()))
                        .copied()
            })
    }

    pub(super) fn dirty(
        &mut self,
        cx: &Cx,
        (root, usable): (NodeId, bool),
        delta: &FrameDelta,
        selection: &HashMap<NodeId, (usize, usize)>,
    ) -> (bool, AHashSet<NodeId>) {
        let mut moved = Vec::new();
        for &id in &delta.layout {
            if cx.get(id).is_some_and(|node| {
                node.extension::<crate::layout::ReactivePosition>()
                    .is_some()
            }) {
                walk_tree(cx, id, |under, _| moved.push(under));
            }
        }
        let touched = delta.layout.len() + delta.paint.len() + moved.len();
        let mass_dirty = !delta.structure && touched * 4 > cx.len();
        if !usable || mass_dirty {
            self.seed_seen(cx, root);
            return (false, AHashSet::new());
        }
        let mut direct = if delta.structure {
            self.dirty_structural(cx, root, delta, selection)
        } else {
            self.dirty_incremental(cx, delta, selection)
        };
        direct.extend(moved);
        (true, ancestors(cx, direct))
    }

    fn seed_seen(&mut self, cx: &Cx, root: NodeId) {
        let seen = self.seen.get_or_insert_with(SecondaryMap::new);
        let watched = &mut self.watched;
        seen.clear();
        watched.clear();
        walk_tree(cx, root, |id, node| {
            let _ = seen.insert(id.key(), seen_of(node));
            if watches(node) {
                watched.push(id);
            }
        });
    }

    fn dirty_structural(
        &mut self,
        cx: &Cx,
        root: NodeId,
        delta: &FrameDelta,
        selection: &HashMap<NodeId, (usize, usize)>,
    ) -> Vec<NodeId> {
        let mut direct: Vec<NodeId> = delta.paint.iter().chain(&delta.layout).copied().collect();
        self.push_selection_diff(selection, &mut direct);
        let seen = self.seen.get_or_insert_with(SecondaryMap::new);
        let watched = &mut self.watched;
        watched.clear();
        walk_tree(cx, root, |id, node| {
            let now = seen_of(node);
            if watches(node) {
                watched.push(id);
            }
            if seen.insert(id.key(), now) != Some(now) {
                direct.push(id);
            }
        });
        direct
    }

    fn dirty_incremental(
        &mut self,
        cx: &Cx,
        delta: &FrameDelta,
        selection: &HashMap<NodeId, (usize, usize)>,
    ) -> Vec<NodeId> {
        let mut direct = Vec::with_capacity(delta.paint.len() + delta.layout.len() + 4);
        self.push_selection_diff(selection, &mut direct);
        let seen = self.seen.get_or_insert_with(SecondaryMap::new);
        for &id in delta.paint.iter().chain(&delta.layout) {
            direct.push(id);
            if let Some(node) = cx.get(id) {
                let _ = seen.insert(id.key(), seen_of(node));
                if watches(node) && !self.watched.contains(&id) {
                    self.watched.push(id);
                }
            }
        }
        for &id in &self.watched {
            if let Some(node) = cx.get(id) {
                let now = seen_of(node);
                if seen.insert(id.key(), now) != Some(now) {
                    direct.push(id);
                }
            }
        }
        direct
    }

    fn push_selection_diff(
        &self,
        selection: &HashMap<NodeId, (usize, usize)>,
        direct: &mut Vec<NodeId>,
    ) {
        if self.selected != *selection {
            direct.extend(self.selected.keys().copied());
            direct.extend(selection.keys().copied());
        }
    }

    pub(super) fn commit(
        &mut self,
        cx: &Cx,
        root: NodeId,
        (spans, len, clip): (Option<SpanMap>, usize, Option<Rect>),
        selection: &HashMap<NodeId, (usize, usize)>,
    ) {
        self.spans = spans;
        if self.selected != *selection {
            self.selected.clone_from(selection);
        }
        self.prev_len = Some(len);
        self.root = Some(root);
        self.scale = cx.scale().to_bits();
        self.clip = clip.map(rect_bits);
        self.theme_epoch = emit_cache::theme_epoch();
    }
}

fn walk_tree(cx: &Cx, root: NodeId, mut visit: impl FnMut(NodeId, &Node)) {
    let mut stack = SmallVec::<[NodeId; 32]>::new();
    stack.push(root);
    while let Some(id) = stack.pop() {
        let Some(node) = cx.get(id) else {
            continue;
        };
        visit(id, node);
        for &child in node.children().iter().rev() {
            stack.push(child);
        }
    }
}

fn ancestors(cx: &Cx, direct: Vec<NodeId>) -> AHashSet<NodeId> {
    if direct.is_empty() {
        return AHashSet::new();
    }
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

fn seen_of(node: &Node) -> Seen {
    Seen {
        version: node.version(),
        layout: node.extension::<LayoutResult>().copied(),
        volatile: volatile_key(node),
    }
}

fn watches(node: &Node) -> bool {
    node.extension::<StyleBinding>()
        .is_some_and(StyleBinding::has_reactive)
        || node.extension::<CanvasPaint>().is_some()
        || node.extension::<FocusRing>().is_some()
}

fn volatile_key(node: &Node) -> (u8, u64, u64, u64, bool) {
    let binding = node.extension::<StyleBinding>();
    let animated = binding.and_then(StyleBinding::animator_overlay).map_or(
        0,
        crate::style::animator_overlay::AnimatorOverlay::cache_key,
    );
    (
        emit_cache::slot_tag(node),
        animated,
        binding.map_or(0, channels_key),
        node.extension::<CanvasPaint>()
            .map_or(0, CanvasPaint::version),
        node.extension::<FocusRing>()
            .is_some_and(FocusRing::visible),
    )
}

fn channels_key(binding: &StyleBinding) -> u64 {
    let mut iter = binding.reactive_channels();
    let Some(first) = iter.next() else {
        return 0;
    };
    let mut hasher = ahash::AHasher::default();
    std::ptr::hash(first, &mut hasher);
    for atoms in iter {
        std::ptr::hash(atoms, &mut hasher);
    }
    std::hash::Hasher::finish(&hasher)
}
