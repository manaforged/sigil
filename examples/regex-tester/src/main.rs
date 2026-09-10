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

#[derive(Clone, Copy)]
struct RegexState {
    pattern: Signal<String>,
    sample: Signal<String>,
    result_label: Signal<String>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct MatchInfo {
    start: usize,
    end: usize,
    text: String,
}

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
