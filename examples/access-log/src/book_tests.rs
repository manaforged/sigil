// The tests index the small book they just allocated; bounds are fixed by construction.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;

#[test]
fn generate_holds_the_book() {
    let book = generate(1_000);
    assert_eq!(book.len(), 1_000);
    assert!(book[0].format(0).contains("GET"));
    assert!(book[1].format(1).contains("/api/v1/items/1"));
}

#[test]
fn tick_moves_visible_bytes() {
    let mut book = generate(200);
    let before = book[0].bytes;
    let (first, end) = window_at(0.0, book.len());
    tick_visible(&mut book, first, end.saturating_sub(first));
    assert!(book[0].bytes > before);
    assert_eq!(book[0].seq, 1);
    assert_eq!(book[70].seq, 1, "overscan below the page must tick");
    assert_eq!(book[190].seq, 0);
}
