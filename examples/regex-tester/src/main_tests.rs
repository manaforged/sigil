#![allow(clippy::indexing_slicing, clippy::unwrap_used)]

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
