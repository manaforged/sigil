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
#[allow(clippy::indexing_slicing, clippy::unwrap_used)]
mod tests {
    use super::*;
    use sigil::reactive::Runtime;

    #[test]
    fn scan_returns_matches_for_valid_pattern() {
        let matches = scan(r"\d+", "abc 123 def 456").expect("compiles");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].text, "123");
        assert_eq!(matches[0].start, 4);
        assert_eq!(matches[0].end, 7);
        assert_eq!(matches[1].text, "456");
    }

    #[test]
    fn scan_returns_empty_for_empty_pattern() {
        let matches = scan("", "anything").expect("empty pattern is ok");
        assert!(matches.is_empty());
    }

    #[test]
    fn scan_returns_err_for_invalid_pattern() {
        // Unbalanced bracket — the regex crate rejects this.
        let err = scan("[unclosed", "anything").expect_err("invalid pattern must error");
        assert!(!err.is_empty(), "error string should carry diagnostic text");
    }

    #[test]
    fn format_matches_renders_one_line_per_match() {
        let body = format_matches(&[
            MatchInfo {
                start: 0,
                end: 3,
                text: "abc".into(),
            },
            MatchInfo {
                start: 5,
                end: 7,
                text: "de".into(),
            },
        ]);
        assert_eq!(body, "[0..3] abc\n[5..7] de\n");
    }

    #[test]
    fn format_matches_handles_empty_list() {
        assert_eq!(format_matches(&[]), "(no matches)");
    }

    #[test]
    fn result_label_updates_when_pattern_changes() {
        Runtime::scope("regex-result-label", || {
            let state = RegexState::new();
            // Default: "\w+" against "hello world 123" → 3 matches.
            assert_eq!(state.result_label.get(), "3 matches");

            state.pattern.set(String::from(r"\d+"));
            assert_eq!(state.result_label.get(), "1 match");

            state.pattern.set(String::from("[unclosed"));
            assert!(
                state.result_label.get().starts_with("Invalid regex:"),
                "compile error must surface in label, got {:?}",
                state.result_label.get()
            );
        });
    }

    #[test]
    fn match_body_updates_when_sample_changes() {
        Runtime::scope("regex-match-body", || {
            let state = RegexState::new();
            state.pattern.set(String::from(r"\d+"));
            state.sample.set(String::from("one 1 two 22 three 333"));

            let body = state.match_body.get();
            assert!(body.contains('1'), "should match `1`: {body}");
            assert!(body.contains("22"), "should match `22`: {body}");
            assert!(body.contains("333"), "should match `333`: {body}");
        });
    }

    #[test]
    fn match_body_is_empty_on_invalid_pattern() {
        Runtime::scope("regex-match-body-invalid", || {
            let state = RegexState::new();
            state.pattern.set(String::from("[bad"));
            assert_eq!(state.match_body.get(), "");
        });
    }

    #[test]
    fn clear_resets_both_inputs() {
        Runtime::scope("regex-clear", || {
            let state = RegexState::new();
            assert_eq!(state.pattern.peek(), PLACEHOLDER_PATTERN);
            state.clear();
            assert_eq!(state.pattern.peek(), "");
            assert_eq!(state.sample.peek(), "");
        });
    }

    #[test]
    fn tree_builds_without_panic() {
        Runtime::scope("regex-tree-smoke", || {
            let mut cx = Cx::new_root();
            let _root = app_tree(&mut cx);
        });
    }
}
