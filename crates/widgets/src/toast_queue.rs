//! Toast queue — stacked, auto-dismissing status toasts (sonner-style).
//!
//! [`ToastQueue`] is a `Copy` handle backed by a signal; push from
//! anywhere in the reactive tree. [`toast_queue`] renders the stack as
//! an absolute overlay pinned bottom-right, newest last, with at most
//! [`MAX_VISIBLE`] entries on screen (older entries stay queued until
//! they expire). Each toast auto-dismisses [`TOAST_TTL`] after push and
//! carries a level-tinted status dot next to the message.
//!
//! Expiry is swept by a `sigil_reactive::register_interval` registered
//! when the widget builds — the host app must call
//! `sigil_reactive::tick_intervals()` once per frame for toasts to
//! expire (the standard reactive runtime already does).
//!
//! ```ignore
//! use sigil::prelude::*;
//!
//! let toasts = ToastQueue::new();
//! toasts.success("Saved");
//! // in the tree: toast_queue(toasts)
//! ```

use std::time::{Duration, Instant};

use sigil_kernel::{NodeBuilder, Role, container, text};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle, Position};
use sigil_reactive::{Signal, register_interval, signal};
use sigil_style::{NodeStyleExt, Tone};

use super::dot::dot;

/// Severity level of a queued toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToastLevel {
    /// Neutral notice — info-tone indicator.
    Info,
    /// Positive confirmation — success-tone indicator.
    Success,
    /// Non-blocking caution — warn-tone indicator.
    Warning,
    /// Failure — error-tone indicator.
    Error,
}

impl ToastLevel {
    /// The [`Tone`] this level's indicator dot resolves through.
    #[must_use]
    pub const fn tone(self) -> Tone {
        match self {
            Self::Info => Tone::Info,
            Self::Success => Tone::Success,
            Self::Warning => Tone::Warn,
            Self::Error => Tone::Error,
        }
    }
}

/// One queued toast: severity, message, and the push timestamp that
/// drives expiry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastEntry {
    /// Severity level, set at push time.
    pub level: ToastLevel,
    /// Message text.
    pub message: String,
    pushed_at: Instant,
}

/// How long after push a toast stays on screen.
pub const TOAST_TTL: Duration = Duration::from_secs(4);
/// Maximum number of toasts rendered at once; older entries stay
/// queued (and expire on schedule) but are not drawn.
pub const MAX_VISIBLE: usize = 4;
/// How often the widget sweeps for expired toasts.
const SWEEP_PERIOD: Duration = Duration::from_millis(250);

/// Clone handle to a stack of toasts. Signal-backed: pushes from any
/// clone re-render every [`toast_queue`] widget bound to the queue.
///
/// Must be created inside a reactive scope (it allocates a signal).
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
    /// Create an empty queue.
    ///
    /// # Panics
    ///
    /// Panics outside a reactive scope, same as
    /// [`sigil_reactive::signal`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: signal(Vec::new()),
        }
    }

    /// Push a toast at `level`. It auto-dismisses [`TOAST_TTL`] later.
    pub fn push(self, level: ToastLevel, message: impl Into<String>) {
        let entry = ToastEntry {
            level,
            message: message.into(),
            pushed_at: Instant::now(),
        };
        self.entries.update(|entries| entries.push(entry));
    }

    /// Push an [`ToastLevel::Info`] toast.
    pub fn info(self, message: impl Into<String>) {
        self.push(ToastLevel::Info, message);
    }

    /// Push a [`ToastLevel::Success`] toast.
    pub fn success(self, message: impl Into<String>) {
        self.push(ToastLevel::Success, message);
    }

    /// Push a [`ToastLevel::Warning`] toast.
    pub fn warning(self, message: impl Into<String>) {
        self.push(ToastLevel::Warning, message);
    }

    /// Push an [`ToastLevel::Error`] toast.
    pub fn error(self, message: impl Into<String>) {
        self.push(ToastLevel::Error, message);
    }

    /// Drop every entry pushed more than [`TOAST_TTL`] before `now`.
    fn dismiss_expired(self, now: Instant) {
        self.entries.update(|entries| {
            entries.retain(|e| now.saturating_duration_since(e.pushed_at) < TOAST_TTL);
        });
    }

    /// The entries to draw: oldest first, newest last, capped at
    /// [`MAX_VISIBLE`] by dropping the oldest.
    fn visible(self) -> Vec<ToastEntry> {
        let entries = self.entries.get();
        let start = entries.len().saturating_sub(MAX_VISIBLE);
        entries.get(start..).unwrap_or(&[]).to_vec()
    }
}

/// Render a [`ToastQueue`] as a stack pinned to the host's bottom-right
/// corner.
///
/// Registers the expiry sweep interval against the current reactive
/// owner, so the stack stops sweeping when its scope disposes.
#[must_use]
pub fn toast_queue(queue: ToastQueue) -> NodeBuilder {
    register_interval(SWEEP_PERIOD, move || queue.dismiss_expired(Instant::now()));
    container()
        .role(Role::Status)
        .layout(stack_layout())
        .children_reactive(move || {
            queue
                .visible()
                .into_iter()
                .map(toast_node)
                .collect::<Vec<_>>()
        })
}

fn stack_layout() -> LayoutStyle {
    LayoutStyle::flex_column()
        .position(Position::Absolute)
        .inset_bottom(16.0)
        .inset_right(16.0)
        .gap(8.0)
        .align_items(AlignItems::End)
}

fn toast_node(entry: ToastEntry) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .align_items(AlignItems::Center)
                .gap(8.0)
                .padding(12.0),
        )
        .class("rounded-md shadow-md bg-surface-hover")
        .child(dot(entry.level.tone()))
        .child(text(entry.message))
}

#[cfg(test)]
#[path = "toast_queue_tests.rs"]
mod tests;
