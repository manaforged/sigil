//! Reactive state for the log-tailer view.

use sigil::prelude::{Signal, signal};

use crate::data::{Level, LogLine, generate};

/// Total number of synthesised log lines.
pub(crate) const LOG_LINE_COUNT: usize = 10_000;

/// Shared state across the log-tailer view.
///
/// All fields are either `Signal<_>` (Copy) or `&'static`, so the whole
/// struct is `Copy`. That's deliberate — it keeps every helper signature
/// pass-by-value without the `needless_pass_by_value` lint firing.
#[derive(Clone, Copy)]
pub(crate) struct AppState {
    /// All synthesised lines (immutable for this read-only viewer).
    pub(crate) all_lines: &'static [LogLine],
    /// Current text filter; matches against the formatted row text
    /// case-insensitively.
    pub(crate) filter_text: Signal<String>,
    /// Currently-selected severity (`None` means show every level).
    pub(crate) level_filter: Signal<Option<Level>>,
    /// Whether the view should auto-scroll to the bottom on every
    /// rebuild. Read-only viewer doesn't append new lines, so this
    /// only fires once during the initial paint to mimic the real
    /// "tail -f" feel.
    pub(crate) auto_scroll: Signal<bool>,
    /// Currently-selected row index in the *filtered* set, or `None`.
    pub(crate) selected: Signal<Option<usize>>,
    /// Reactive label of the visible row count for the status bar.
    pub(crate) visible_label: Signal<String>,
}

impl AppState {
    /// Build a fresh state with the default corpus.
    pub(crate) fn new() -> Self {
        let all_lines: &'static [LogLine] = Box::leak(generate(LOG_LINE_COUNT).into_boxed_slice());
        Self {
            all_lines,
            filter_text: signal(String::new()),
            level_filter: signal(None),
            auto_scroll: signal(true),
            selected: signal(None),
            visible_label: signal(format!("{LOG_LINE_COUNT} / {LOG_LINE_COUNT} lines")),
        }
    }

    /// Apply the current filters and return the matching rows.
    ///
    /// Iterates the static corpus once per call; cheap enough for 10k
    /// items in a viewer context. A production tailer would cache the
    /// filtered indices in a `derived` signal.
    pub(crate) fn filtered(self) -> Vec<LogLine> {
        let needle = self.filter_text.peek().to_lowercase();
        let level = self.level_filter.peek();
        self.all_lines
            .iter()
            .filter(|line| level.is_none_or(|wanted| line.level == wanted))
            .filter(|line| {
                if needle.is_empty() {
                    return true;
                }
                line.message.to_lowercase().contains(&needle)
                    || line.level.label().to_lowercase().contains(&needle)
            })
            .cloned()
            .collect()
    }

    /// Update the reactive `visible_label` to match the current
    /// filter result count.
    pub(crate) fn refresh_visible_label(self, visible: usize) {
        let total = self.all_lines.len();
        self.visible_label.set(format!("{visible} / {total} lines"));
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
