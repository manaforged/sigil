#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;

#[test]
fn exact_match_short_circuits() {
    let (s, p) = score_entry("main", "main").unwrap();
    assert_eq!(s, score::EXACT);
    assert_eq!(p.as_slice(), &[0, 1, 2, 3]);
}

#[test]
fn prefix_match_short_circuits() {
    let (s, _) = score_entry("main", "main.rs").unwrap();
    assert_eq!(s, score::PREFIX);
}

#[test]
fn icase_prefix_below_csens_prefix() {
    let (cs, _) = score_entry("main", "main.rs").unwrap();
    let (ic, _) = score_entry("main", "Main.rs").unwrap();
    assert!(cs > ic, "case-sensitive prefix must outrank icase prefix");
}

#[test]
fn subsequence_no_match_returns_none() {
    assert!(score_entry("xyz", "abcdef").is_none());
}

#[test]
fn empty_haystack_returns_none() {
    assert!(score_entry("a", "").is_none());
}
