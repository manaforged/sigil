//! Regex tester — a tier-3 dogfood example.
//!
//! Two text fields: a regex pattern + a sample text body. A derived
//! signal compiles the pattern on every change; if compilation succeeds
//! it scans the sample and emits a `Vec<MatchInfo>`. If compilation
//! fails the error string is surfaced inline (no panic — `Regex::new`
//! returns a `Result`).
//!
//! The result is rendered three ways so the dogfood pass exercises the
//! reactive surface end-to-end:
//!
//! 1. A status label — "3 matches" / "Invalid regex: …" — bound to a
//!    derived `Signal<String>`.
//! 2. A formatted match list — rendered as a single derived text body
//!    (one line per match: byte range + matched substring) because
//!    `virtual_list` takes `Vec<T>` by value at build time and is
//!    therefore non-reactive to list growth (see `FRICTION.md` §
//!    "`virtual_list` is not reactive to items").
//! 3. A "Clear" button that resets both fields to empty.
//!
//! See `FRICTION.md` for the friction log on the API surface this
//! exercised.
//!
//! The sample text uses single-line `text_field` rather than the
//! multi-line `text_edit` widget — see FRICTION § "`text_edit` line
//! children freeze at build" for the limitation that pushed me here.

use regex::Regex;
use sigil::prelude::*;

const PLACEHOLDER_PATTERN: &str = r"\w+";
const PLACEHOLDER_SAMPLE: &str = "hello world 123";

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - regex-tester", 720, 480), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    let state = RegexState::new();
    cx.add(root_view(state))
}

/// Reactive state for the regex tester.
///
/// `pattern` and `sample` are user-editable inputs. `result_label` and
/// `match_body` are derived from those two inputs and surface the
/// compile/scan outcome to the view.
#[derive(Clone, Copy)]
struct RegexState {
    pattern: Signal<String>,
    sample: Signal<String>,
    /// Either "N match(es)" or "Invalid regex: `<error>`."
    result_label: Signal<String>,
    /// One line per match, formatted "[start..end] `matched_text`".
    /// Empty string when there are no matches or the pattern is invalid.
    match_body: Signal<String>,
}

impl RegexState {
    fn new() -> Self {
        let pattern = signal(String::from(PLACEHOLDER_PATTERN));
        let sample = signal(String::from(PLACEHOLDER_SAMPLE));

        let result_label = derived(move || match scan(&pattern.get(), &sample.get()) {
            Ok(matches) => {
                if matches.len() == 1 {
                    String::from("1 match")
                } else {
                    format!("{} matches", matches.len())
                }
            }
            Err(err) => format!("Invalid regex: {err}"),
        });

        let match_body = derived(move || {
            scan(&pattern.get(), &sample.get())
                .map_or_else(|_| String::new(), |matches| format_matches(&matches))
        });

        Self {
            pattern,
            sample,
            result_label,
            match_body,
        }
    }

    fn clear(self) {
        self.pattern.set(String::new());
        self.sample.set(String::new());
    }
}

/// A single regex match: byte range + captured slice.
#[derive(Clone, Debug, PartialEq, Eq)]
struct MatchInfo {
    start: usize,
    end: usize,
    text: String,
}

/// Compile `pattern` and find all matches in `sample`.
///
/// Returns `Err` with a humanised error string if the pattern fails to
/// compile. Returns an empty `Vec` if it compiles but matches nothing.
fn scan(pattern: &str, sample: &str) -> core::result::Result<Vec<MatchInfo>, String> {
    if pattern.is_empty() {
        return Ok(Vec::new());
    }
    match Regex::new(pattern) {
        Ok(re) => Ok(re
            .find_iter(sample)
            .map(|m| MatchInfo {
                start: m.start(),
                end: m.end(),
                text: m.as_str().to_owned(),
            })
            .collect()),
        Err(err) => Err(err.to_string()),
    }
}

/// Render a list of matches as a newline-joined block.
fn format_matches(matches: &[MatchInfo]) -> String {
    if matches.is_empty() {
        return String::from("(no matches)");
    }
    let mut out = String::with_capacity(matches.len() * 32);
    for m in matches {
        out.push('[');
        out.push_str(&m.start.to_string());
        out.push_str("..");
        out.push_str(&m.end.to_string());
        out.push_str("] ");
        out.push_str(&m.text);
        out.push('\n');
    }
    out
}

fn root_view(state: RegexState) -> NodeBuilder {
    container()
        .role(Role::Main)
        .label("Sigil regex tester")
        .layout(LayoutStyle::flex_column().gap(12.0).padding(24.0).grow(1.0))
        .class(classes!("w-full h-full bg-surface"))
        .child(header_row(state))
        .child(pattern_row(state))
        .child(sample_row(state))
        .child(results_row(state))
        .child(footer_row(state))
}

fn header_row(state: RegexState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(12.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween),
        )
        .child(label("Regex tester").class(classes!("text-on-surface text-lg font-bold")))
        .child(label(state.result_label).class(classes!("text-on-surface")))
}

fn pattern_row(state: RegexState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(4.0))
        .child(label("Pattern").class(classes!("text-on-surface text-sm")))
        .child(text_field(state.pattern).placeholder(r"e.g. \w+ or \d{3}-\d{4}"))
}

fn sample_row(state: RegexState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(4.0))
        .child(label("Sample text").class(classes!("text-on-surface text-sm")))
        .child(text_field(state.sample).placeholder("Text to scan…"))
}

fn results_row(state: RegexState) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .label("Match results")
        .layout(LayoutStyle::flex_column().gap(4.0).grow(1.0))
        .class(classes!("p-2 bg-surface"))
        .child(label("Matches").class(classes!("text-on-surface text-sm")))
        .child(
            text(state.match_body)
                .layout(LayoutStyle::block().width(672.0))
                .class(classes!("text-on-surface font-mono text-sm")),
        )
}

fn footer_row(state: RegexState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center),
        )
        .child(button("Clear").activate(move || state.clear()))
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
