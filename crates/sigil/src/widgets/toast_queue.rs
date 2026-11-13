use crate::layout::Space;
use std::time::Duration;
use web_time::Instant;

use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::{AlignItems, LayoutStyle, Position};
use crate::reactive::{Signal, interval, signal};
use crate::style::{Token, Tone, space};
use crate::widgets::Widget;

use super::dot::dot;
use super::popup::SURFACE;

const SWEEP_PERIOD: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToastEntry {
    pub(crate) tone: Tone,
    pub(crate) message: String,
    shown_at: Option<Instant>,
}

const TOAST_TTL: Duration = Duration::from_secs(4);
const MAX_VISIBLE: usize = 4;

/// Queue of toast messages.
#[derive(Debug, Clone, Copy)]
pub struct ToastQueue {
    entries: Signal<Vec<ToastEntry>>,
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
        }
    }

    /// Shows a toast in `tone`.
    pub fn push(self, tone: Tone, message: impl Into<String>) {
        let entry = ToastEntry {
            tone,
            message: message.into(),
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
        if !self.entries.peek().iter().any(expired) {
            return;
        }
        self.entries.update(|entries| {
            entries.retain(|entry| !expired(entry));
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

/// A stack of transient messages.
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
                .align_items(AlignItems::End),
        )
        .children_reactive(move || {
            queue
                .visible()
                .into_iter()
                .map(|entry| {
                    container()
                        .layout(
                            LayoutStyle::flex_row()
                                .align_items(AlignItems::Center)
                                .gap(Space::Sm)
                                .padding(Space::Md),
                        )
                        .class(SURFACE)
                        .child(dot(entry.tone))
                        .child(text(entry.message).class(Token::OnSurface.text()))
                })
                .collect::<Vec<_>>()
        })
}

#[cfg(test)]
#[path = "toast_queue_tests.rs"]
mod tests;
