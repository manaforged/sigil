#[cfg(terminal_host)]
use core::ops::Range;
use std::rc::Rc;

use ahash::{AHashMap, AHashSet};

use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::profile::{Counter, count_add};
use crate::scene::command::SceneCommand;
use crate::scene::emit_cache;
use crate::style::{Color, TextStyle};

use super::{EmitWalk, Inherited};

#[derive(Clone)]
pub(crate) struct SubtreeSpan {
    start: usize,
    len: usize,
    nodes: u64,
    key: SpanKey,
}

#[derive(Clone)]
struct SpanKey {
    theme: u64,
    text_color: Option<Color>,
    text_style: Rc<TextStyle>,
    clip: Option<[u32; 4]>,
    fill: u64,
}

impl SpanKey {
    fn of(inherited: &Inherited<'_>, clip: Option<Rect>) -> Self {
        Self {
            theme: inherited.theme_stamp ^ emit_cache::theme_stamp(),
            text_color: inherited.text_color,
            text_style: Rc::clone(&inherited.text_style),
            clip: clip.map(rect_bits),
            fill: fill_key(inherited),
        }
    }

    fn matches(&self, inherited: &Inherited<'_>, clip: Option<Rect>) -> bool {
        self.theme == inherited.theme_stamp ^ emit_cache::theme_stamp()
            && self.text_color == inherited.text_color
            && self.clip == clip.map(rect_bits)
            && self.fill == fill_key(inherited)
            && (Rc::ptr_eq(&self.text_style, &inherited.text_style)
                || *self.text_style == *inherited.text_style)
    }
}

fn fill_key(inherited: &Inherited<'_>) -> u64 {
    inherited.text_fill.as_ref().map_or(0, |fill| fill.key)
        ^ emit_cache::shadow_key(inherited.text_shadows.as_deref().unwrap_or_default())
}

pub(super) const fn rect_bits(rect: Rect) -> [u32; 4] {
    [
        rect.origin.x.to_bits(),
        rect.origin.y.to_bits(),
        rect.width.to_bits(),
        rect.height.to_bits(),
    ]
}

#[cfg(terminal_host)]
kittest_pub! {
    /// What `Host::emit_since` changed: pairs of old and new command ranges that differ.
    #[doc(hidden)]
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct EmitDiff {
        pub(crate) changed: Vec<(Range<usize>, Range<usize>)>,
    }
}

#[cfg(terminal_host)]
impl EmitDiff {
    kittest_pub! {
        /// Returns whether unchanged is set.
        #[must_use]
        pub const fn unchanged(&self) -> bool {
            self.changed.is_empty()
        }
    }
}

#[cfg(terminal_host)]
#[derive(Clone, Copy)]
pub(super) struct Copied {
    pub(super) old: usize,
    pub(super) new: usize,
    pub(super) len: usize,
}

#[derive(Clone, Copy)]
pub(super) struct Prior<'t> {
    pub(super) commands: &'t [SceneCommand],
    pub(super) spans: &'t AHashMap<NodeId, SubtreeSpan>,
    pub(super) dirty: &'t AHashSet<NodeId>,
}

#[derive(Default)]
pub(super) struct SpanTrack<'t> {
    prior: Option<Prior<'t>>,
    record: bool,
    nodes: u64,
    pub(super) next: AHashMap<NodeId, SubtreeSpan>,
    #[cfg(terminal_host)]
    pub(super) copies: Vec<Copied>,
}

pub(super) struct Open {
    start: usize,
    deferred: usize,
    nodes: u64,
}

#[cfg(terminal_host)]
impl<'t> SpanTrack<'t> {
    pub(super) fn recording(prior: Option<Prior<'t>>) -> Self {
        Self {
            prior,
            record: true,
            ..Self::default()
        }
    }
}

impl SpanTrack<'_> {
    pub(super) fn off() -> Self {
        Self::default()
    }

    pub(super) const fn walked(&mut self) {
        self.nodes += 1;
    }
}

impl<'t> EmitWalk<'_, 't> {
    pub(super) fn copy_subtree(
        &mut self,
        id: NodeId,
        inherited: &Inherited<'t>,
        clip: Option<Rect>,
    ) -> bool {
        let Some(prior) = self.track.prior else {
            return false;
        };
        if prior.dirty.contains(&id) {
            return false;
        }
        let Some(span) = prior
            .spans
            .get(&id)
            .filter(|span| span.key.matches(inherited, clip))
        else {
            return false;
        };
        let Some(commands) = prior.commands.get(span.start..span.start + span.len) else {
            return false;
        };
        let start = self.scene.command_len();
        self.scene.extend_commands(commands.iter().cloned());
        count_add(Counter::EmitCopied, span.nodes);
        self.track.nodes += span.nodes;
        #[cfg(terminal_host)]
        self.track.copies.push(Copied {
            old: span.start,
            new: start,
            len: span.len,
        });
        let next = &mut self.track.next;
        let (old, end) = (span.start, span.start + span.len);
        self.cx.visit_preorder(id, |inner, _| {
            if let Some(kept) = prior
                .spans
                .get(&inner)
                .filter(|kept| kept.start >= old && kept.start + kept.len <= end)
            {
                next.insert(
                    inner,
                    SubtreeSpan {
                        start: kept.start - old + start,
                        ..kept.clone()
                    },
                );
            }
        });
        true
    }

    pub(super) const fn open_span(&self) -> Open {
        Open {
            start: self.scene.command_len(),
            deferred: self.deferred.len(),
            nodes: self.track.nodes,
        }
    }

    pub(super) fn close_span(
        &mut self,
        id: NodeId,
        inherited: &Inherited<'t>,
        clip: Option<Rect>,
        open: &Open,
    ) {
        if !self.track.record || self.deferred.len() != open.deferred {
            return;
        }
        let span = SubtreeSpan {
            start: open.start,
            len: self.scene.command_len() - open.start,
            nodes: self.track.nodes - open.nodes,
            key: SpanKey::of(inherited, clip),
        };
        self.track.next.insert(id, span);
    }
}
