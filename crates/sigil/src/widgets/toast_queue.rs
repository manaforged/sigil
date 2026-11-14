use crate::layout::Space;
use std::time::Duration;
use web_time::Instant;

use crate::animation::{Easing, Property, Transition};
use crate::kernel::{NodeBuilder, Role, container, for_each, text};
use crate::layout::{AlignItems, LayoutStyle, Position};
use crate::reactive::{Signal, interval, signal};
use crate::style::{Paint, Token, Tone, space};
use crate::widgets::swipe::Swipe;
use crate::widgets::{Side, Widget};

use super::dot::dot;
use super::popup::SURFACE;

const SWEEP_PERIOD: Duration = Duration::from_millis(250);
const MOTION: Duration = Duration::from_millis(220);
const RISE: f32 = space(4);
const SHRINK: f32 = 0.96;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToastEntry {
    pub(crate) tone: Tone,
    pub(crate) message: String,
    id: u64,
    shown_at: Option<Instant>,
}

const TOAST_TTL: Duration = Duration::from_secs(4);
const MAX_VISIBLE: usize = 4;

/// Queue of toast messages.
#[derive(Debug, Clone, Copy)]
pub struct ToastQueue {
    entries: Signal<Vec<ToastEntry>>,
    next_id: Signal<u64>,
}

impl Default for ToastQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastQueue {
    /// Creates a toast queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: signal(Vec::new()),
            next_id: signal(0),
        }
    }

    /// Shows a toast in `tone`.
    pub fn push(self, tone: Tone, message: impl Into<String>) {
        let id = self.next_id.peek();
        self.next_id.set(id.wrapping_add(1));
        let entry = ToastEntry {
            tone,
            message: message.into(),
            id,
            shown_at: None,
        };
        let now = Instant::now();
        self.entries.update(|entries| {
            entries.push(entry);
            stamp_visible(entries, now);
        });
    }

    /// Shows an information toast.
    pub fn info(self, message: impl Into<String>) {
        self.push(Tone::Info, message);
    }

    /// Shows a success toast.
    pub fn success(self, message: impl Into<String>) {
        self.push(Tone::Success, message);
    }

    /// Shows a warning toast.
    pub fn warning(self, message: impl Into<String>) {
        self.push(Tone::Warning, message);
    }

    /// Shows an error toast.
    pub fn error(self, message: impl Into<String>) {
        self.push(Tone::Error, message);
    }

    fn dismiss_expired(self, now: Instant) {
        let expired = |entry: &ToastEntry| {
            entry
                .shown_at
                .is_some_and(|at| now.saturating_duration_since(at) >= TOAST_TTL)
        };
        self.remove_where(expired, now);
    }

    fn dismiss(self, id: u64) {
        self.remove_where(|entry| entry.id == id, Instant::now());
    }

    fn remove_where(self, gone: impl Fn(&ToastEntry) -> bool, now: Instant) {
        if !self.entries.peek().iter().any(&gone) {
            return;
        }
        self.entries.update(|entries| {
            entries.retain(|entry| !gone(entry));
            stamp_visible(entries, now);
        });
    }

    fn visible(self) -> Vec<ToastEntry> {
        let entries = self.entries.get();
        let start = entries.len().saturating_sub(MAX_VISIBLE);
        entries.get(start..).unwrap_or(&[]).to_vec()
    }
}

fn stamp_visible(entries: &mut [ToastEntry], now: Instant) {
    let start = entries.len().saturating_sub(MAX_VISIBLE);
    for (index, entry) in entries.iter_mut().enumerate() {
        if index < start {
            entry.shown_at = None;
        } else {
            entry.shown_at.get_or_insert(now);
        }
    }
}

/// A stack of transient messages that rise in, fade out, and leave when swiped right.
pub fn toast_queue(queue: ToastQueue) -> NodeBuilder {
    interval(SWEEP_PERIOD, move || {
        queue.dismiss_expired(Instant::now());
    });
    container()
        .role(Role::Status)
        .layout(
            LayoutStyle::flex_column()
                .position(Position::Absolute)
                .inset_bottom(space(4))
                .inset_right(space(4))
                .gap(Space::Sm)
                .align_items(AlignItems::Stretch),
        )
        .child(for_each(
            move || queue.visible(),
            |entry| entry.id,
            move |entry| toast_row(queue, entry),
        ))
}

fn toast_row(queue: ToastQueue, entry: &ToastEntry) -> NodeBuilder {
    let id = entry.id;
    let row = container()
        .layout(
            LayoutStyle::flex_row()
                .stretch_width()
                .align_items(AlignItems::Center)
                .gap(Space::Sm)
                .padding(Space::Md),
        )
        .class(SURFACE)
        .transition(
            Transition::new()
                .duration(MOTION)
                .ease(Easing::CubicOut)
                .layout(),
        )
        .enter(Property::Opacity(0.0))
        .enter(Property::TranslateY(RISE))
        .exit(Property::Opacity(0.0))
        .exit(Property::Scale(SHRINK))
        .child(dot(entry.tone))
        .child(text(entry.message.clone()).class(Token::OnSurface.text()));
    Swipe::new(Side::Right).attach(row, move || queue.dismiss(id))
}

#[cfg(test)]
#[path = "toast_queue_tests.rs"]
mod tests;
