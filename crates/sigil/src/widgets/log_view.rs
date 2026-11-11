use std::rc::Rc;

use crate::kernel::text;
use crate::layout::rows;
use crate::reactive::{Signal, signal};

use super::virtual_list::{RowRender, VirtualListBuilder, virtual_list};

#[derive(Clone, Debug, Default)]
struct Lines {
    buffer: Vec<String>,
    start: usize,
    max: usize,
}

impl Lines {
    fn push(&mut self, line: String) {
        self.buffer.push(line);
        if self.buffer.len() - self.start > self.max {
            self.start += 1;
        }
        if self.start >= self.max {
            self.buffer.drain(..self.start);
            self.start = 0;
        }
    }
}

impl AsRef<[String]> for Lines {
    fn as_ref(&self) -> &[String] {
        self.buffer.get(self.start..).unwrap_or_default()
    }
}

/// The lines of a log view: append-only, holding at most a set number of lines.
///
/// A handle is cheap to copy. Write it on the UI thread.
#[derive(Clone, Copy, Debug)]
pub struct Log {
    lines: Signal<Rc<Lines>>,
}

impl Log {
    /// Creates an empty log that keeps the newest `max_lines` lines.
    #[must_use]
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: signal(Rc::new(Lines {
                buffer: Vec::new(),
                start: 0,
                max: max_lines.max(1),
            })),
        }
    }

    /// Appends text as lines split at `\n` or `\r\n`; empty text adds one empty line, a trailing break adds none, and the oldest lines drop past the limit.
    pub fn push(&self, text: impl AsRef<str>) {
        let text = text.as_ref();
        self.lines.update(|lines| {
            let lines = Rc::make_mut(lines);
            if text.is_empty() {
                lines.push(String::new());
            }
            for line in text.lines() {
                lines.push(line.to_owned());
            }
        });
    }

    /// Removes every line.
    pub fn clear(&self) {
        self.lines.update(|lines| {
            let lines = Rc::make_mut(lines);
            lines.buffer.clear();
            lines.start = 0;
        });
    }

    /// Returns the number of lines held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.with(|lines| lines.as_ref().as_ref().len())
    }

    /// Returns whether the log holds no lines.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A scrolling view of a [`Log`] that follows new lines until the user scrolls up.
///
/// Scrolling back to the end resumes following. The result is a [`virtual_list`], so its
/// setters, such as `viewport_height` and `scroll_bind`, apply.
///
/// ```
/// use sigil::prelude::*;
/// use sigil::widgets::{Log, log_view};
///
/// fn server_log() -> impl IntoNode {
///     let log = Log::new(1_000);
///     log.push("server started");
///     column().child(log_view(log).viewport_height(240.0))
/// }
/// ```
pub fn log_view(log: Log) -> VirtualListBuilder<String, impl RowRender<String>> {
    virtual_list(log.lines, |_, line: &String| {
        text(line.clone()).nowrap().class("font-mono text-sm")
    })
    .row_height(rows(1.0))
    .follow_end(true)
}
