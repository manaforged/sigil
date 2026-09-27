use std::time::Duration;
use web_time::Instant;

use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle, Position};
use crate::reactive::{Signal, register_interval, signal};
use crate::style::{NodeStyleExt, Token, Tone, space};

use super::dot::dot;
use super::popup::SURFACE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastLevel {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastEntry {
    pub level: ToastLevel,
    pub message: String,
    pushed_at: Instant,
}

pub const TOAST_TTL: Duration = Duration::from_secs(4);
pub const MAX_VISIBLE: usize = 4;
const SWEEP_PERIOD: Duration = Duration::from_millis(250);

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
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: signal(Vec::new()),
        }
    }

    pub fn push(self, level: ToastLevel, message: impl Into<String>) {
        let entry = ToastEntry {
            level,
            message: message.into(),
            pushed_at: Instant::now(),
        };
        self.entries.update(|entries| entries.push(entry));
    }

    pub fn info(self, message: impl Into<String>) {
        self.push(ToastLevel::Info, message);
    }

    pub fn success(self, message: impl Into<String>) {
        self.push(ToastLevel::Success, message);
    }

    pub fn warning(self, message: impl Into<String>) {
        self.push(ToastLevel::Warning, message);
    }

    pub fn error(self, message: impl Into<String>) {
        self.push(ToastLevel::Error, message);
    }

    fn dismiss_expired(self, now: Instant) {
        self.entries.update(|entries| {
            entries.retain(|e| now.saturating_duration_since(e.pushed_at) < TOAST_TTL);
        });
    }

    fn visible(self) -> Vec<ToastEntry> {
        let entries = self.entries.get();
        let start = entries.len().saturating_sub(MAX_VISIBLE);
        entries.get(start..).unwrap_or(&[]).to_vec()
    }
}

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
        .inset_bottom(space(4))
        .inset_right(space(4))
        .gap(space(2))
        .align_items(AlignItems::End)
}

fn toast_node(entry: ToastEntry) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .align_items(AlignItems::Center)
                .gap(space(2))
                .padding(space(3)),
        )
        .class(SURFACE)
        .child(dot(entry.level.tone()))
        .child(text(entry.message).class(Token::OnSurface.text()))
}

#[cfg(test)]
#[path = "toast_queue_tests.rs"]
mod tests;
