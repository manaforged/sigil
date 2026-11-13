#[cfg(terminal_host)]
use core::ops::Range;

use ahash::AHashSet;
use slotmap::{DefaultKey, SecondaryMap};
use smallvec::SmallVec;

use crate::geometry::Rect;
use crate::kernel::NodeId;
use crate::profile::{Counter, count};
use crate::scene::command::SceneCommand;
use crate::scene::emit_cache;
use crate::style::Color;

use super::{EmitWalk, Inherited};

#[derive(Clone, Copy)]
pub(crate) struct SubtreeSpan {
    start: usize,
    len: usize,
    nodes: u64,
    key: SpanKey,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SpanKey {
    theme: u64,
    text_color: Option<Color>,
    text_style: u64,
    clip: Option<[u32; 4]>,
    fill: u64,
}

impl SpanKey {
    fn of(inherited: &Inherited<'_>, clip: Option<Rect>) -> Self {
        Self {
            theme: inherited.theme_stamp ^ emit_cache::theme_stamp(),
            text_color: inherited.text_color,
            text_style: inherited.text_style_stamp,
            clip: clip.map(rect_bits),
            fill: fill_key(inherited),
        }
    }

    fn matches(&self, inherited: &Inherited<'_>, clip: Option<Rect>) -> bool {
        self.theme == inherited.theme_stamp ^ emit_cache::theme_stamp()
            && self.text_color == inherited.text_color
            && self.text_style == inherited.text_style_stamp
            && self.clip == clip.map(rect_bits)
            && self.fill == fill_key(inherited)
    }
}

fn fill_key(inherited: &Inherited<'_>) -> u64 {
    inherited.text_fill.as_ref().map_or(0, |fill| fill.key)
        ^ emit_cache::shadow_key(inherited.text_shadows.as_deref().unwrap_or_default())
}

pub(super) use crate::scene::emit_cache::rect_bits;

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

pub(super) type SpanMap = SecondaryMap<DefaultKey, SubtreeSpan>;

#[derive(Clone, Copy)]
pub(super) struct Prior<'t> {
    pub(super) commands: &'t [SceneCommand],
    pub(super) spans: &'t SpanMap,
    pub(super) dirty: &'t AHashSet<NodeId>,
}

pub(super) struct SpanTrack<'t> {
    prior: Option<Prior<'t>>,
    #[cfg(terminal_host)]
    want_diff: bool,
    nodes: u64,
    pub(super) next: Option<SpanMap>,
    #[cfg(terminal_host)]
    pub(super) copies: Vec<Copied>,
}

pub(super) struct Open {
    start: usize,
    deferred: usize,
    nodes: u64,
}

#[cfg(host)]
impl<'t> SpanTrack<'t> {
    pub(super) const fn recording(
        prior: Option<Prior<'t>>,
        next: SpanMap,
        want_diff: bool,
    ) -> Self {
        #[cfg(not(terminal_host))]
        let _ = want_diff;
        Self {
            prior,
            #[cfg(terminal_host)]
            want_diff,
            nodes: 0,
            next: Some(next),
            #[cfg(terminal_host)]
            copies: Vec::new(),
        }
    }
}

impl SpanTrack<'_> {
    pub(super) const fn off() -> Self {
        Self {
            prior: None,
            #[cfg(terminal_host)]
            want_diff: false,
            nodes: 0,
            next: None,
            #[cfg(terminal_host)]
            copies: Vec::new(),
        }
    }

    #[cfg(terminal_host)]
    pub(super) const fn with_copies(copies: Vec<Copied>) -> Self {
        Self {
            prior: None,
            want_diff: false,
            nodes: 0,
            next: None,
            copies,
        }
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
            .get(id.key())
            .filter(|span| span.key.matches(inherited, clip))
        else {
            return false;
        };
        let Some(commands) = prior.commands.get(span.start..span.start + span.len) else {
            return false;
        };
        let start = self.scene.command_len();
        self.scene.extend_commands(commands.iter().cloned());
        count(Counter::EmitCopied, span.nodes);
        self.track.nodes += span.nodes;
        #[cfg(terminal_host)]
        if self.track.want_diff {
            self.track.copies.push(Copied {
                old: span.start,
                new: start,
                len: span.len,
            });
        }
        self.remap_copied_spans(id, prior, (span.start, span.len, start));
        true
    }

    fn remap_copied_spans(
        &mut self,
        id: NodeId,
        prior: Prior<'t>,
        (old, len, start): (usize, usize, usize),
    ) {
        let Some(next) = &mut self.track.next else {
            return;
        };
        let end = old + len;
        let mut stack = SmallVec::<[NodeId; 16]>::new();
        stack.push(id);
        while let Some(inner) = stack.pop() {
            if let Some(kept) = prior
                .spans
                .get(inner.key())
                .copied()
                .filter(|kept| kept.start >= old && kept.start + kept.len <= end)
            {
                let _ = next.insert(
                    inner.key(),
                    SubtreeSpan {
                        start: kept.start - old + start,
                        ..kept
                    },
                );
            }
            if let Some(node) = self.cx.get(inner) {
                for &child in node.children().iter().rev() {
                    stack.push(child);
                }
            }
        }
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
        if self.deferred.len() != open.deferred {
            return;
        }
        let len = self.scene.command_len() - open.start;
        let nodes = self.track.nodes - open.nodes;
        let Some(next) = &mut self.track.next else {
            return;
        };
        let span = SubtreeSpan {
            start: open.start,
            len,
            nodes,
            key: SpanKey::of(inherited, clip),
        };
        let _ = next.insert(id.key(), span);
    }
}
