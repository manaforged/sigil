#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;

#[test]
fn empty_needle_returns_input_order() {
    let hay = ["alpha", "beta", "gamma"];
    let out = fuzzy("", &hay);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].idx, 0);
    assert_eq!(out[1].idx, 1);
    assert_eq!(out[2].idx, 2);
    for m in &out {
        assert!(m.positions.is_empty());
        assert_eq!(m.score, 0);
    }
}

#[test]
fn empty_needle_respects_limit() {
    let hay = ["a", "b", "c", "d"];
    let out = fuzzy_with("", &hay, 2);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].idx, 0);
    assert_eq!(out[1].idx, 1);
}

#[test]
fn zero_limit_returns_empty() {
    let hay = ["alpha"];
    assert!(fuzzy_with("a", &hay, 0).is_empty());
    assert!(fuzzy_with("", &hay, 0).is_empty());
}

#[test]
fn single_char_needle_does_not_panic() {
    let hay = ["", "a", "ab", "no-match-here"];
    let _ = fuzzy("a", &hay);
    let _ = fuzzy("z", &hay);
    let _ = fuzzy("/", &hay);
}

#[test]
fn exact_beats_prefix_beats_substring() {
    let hay = ["main", "main.rs", "src/main.rs", "remaining"];
    let out = fuzzy("main", &hay);
    assert_eq!(out[0].idx, 0);
    let main_rs = out.iter().position(|m| m.idx == 1).unwrap();
    let remaining = out.iter().position(|m| m.idx == 3).unwrap();
    assert!(
        main_rs < remaining,
        "prefix `main.rs` should outrank substring `remaining`: out = {out:?}"
    );
}

#[test]
fn case_sensitive_outranks_case_insensitive() {
    let hay = ["Foo", "foo"];
    let out = fuzzy("foo", &hay);
    assert_eq!(out[0].idx, 1, "case-sensitive exact `foo` wins over `Foo`");
}

#[test]
fn camelcase_boundary_boost() {
    let hay = ["coca", "createObject"];
    let out = fuzzy("cO", &hay);
    assert_eq!(out[0].idx, 1);
}

#[test]
fn path_segment_boost() {
    let hay = ["amalgam-light", "src/main/lib.rs"];
    let out = fuzzy("ml", &hay);
    assert_eq!(out[0].idx, 1);
}

#[test]
fn non_ascii_handled() {
    let hay = ["café-menu", "naïve.txt", "résumé/cover.md", "plain.txt"];
    let out = fuzzy("cm", &hay);
    assert!(!out.is_empty());
    let out2 = fuzzy("é", &hay);
    assert!(!out2.is_empty());
}

#[test]
fn no_match_returns_empty() {
    let hay = ["alpha", "beta", "gamma"];
    let out = fuzzy("zzz", &hay);
    assert!(out.is_empty());
}

#[test]
fn limit_truncates_after_sort() {
    let hay = ["main", "main.rs", "src/main.rs", "manifest", "remaining"];
    let out = fuzzy_with("main", &hay, 2);
    assert_eq!(out.len(), 2);
    assert!(out.iter().any(|m| m.idx == 0));
}

#[test]
fn positions_are_ascending_and_in_bounds() {
    let hay = ["src/main.rs"];
    let out = fuzzy("smr", &hay);
    assert_eq!(out.len(), 1);
    let m = &out[0];
    let len = hay[0].len();
    let mut last = 0usize;
    for (i, &p) in m.positions.iter().enumerate() {
        assert!(p < len);
        if i > 0 {
            assert!(p > last);
        }
        last = p;
    }
}

#[test]
fn monotone_dense_beats_sparse() {
    let dense = "abc-something-else";
    let sparse = "a-x-b-x-c-something-else";
    let hay = [dense, sparse];
    let out = fuzzy("abc", &hay);
    assert_eq!(out.len(), 2);
    let dense_score = out.iter().find(|m| m.idx == 0).unwrap().score;
    let sparse_score = out.iter().find(|m| m.idx == 1).unwrap().score;
    assert!(
        dense_score >= sparse_score,
        "dense {dense_score} should be >= sparse {sparse_score}"
    );
}

#[test]
fn deterministic_tiebreak_by_idx() {
    let hay = ["same", "same", "same"];
    let out = fuzzy("same", &hay);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].idx, 0);
    assert_eq!(out[1].idx, 1);
    assert_eq!(out[2].idx, 2);
}
