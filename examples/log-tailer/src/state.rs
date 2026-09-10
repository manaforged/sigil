use sigil::prelude::{Signal, signal};

use crate::data::{Level, LogLine, generate};

pub(crate) const LOG_LINE_COUNT: usize = 10_000;

#[derive(Clone, Copy)]
pub(crate) struct AppState {
    pub(crate) all_lines: &'static [LogLine],
    pub(crate) filter_text: Signal<String>,
    pub(crate) level_filter: Signal<Option<Level>>,
    pub(crate) auto_scroll: Signal<bool>,
    pub(crate) selected: Signal<Option<usize>>,
    pub(crate) visible_label: Signal<String>,
}

impl AppState {
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

    pub(crate) fn refresh_visible_label(self, visible: usize) {
        let total = self.all_lines.len();
        self.visible_label.set(format!("{visible} / {total} lines"));
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
