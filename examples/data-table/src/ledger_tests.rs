#![allow(clippy::indexing_slicing)]

use super::*;

#[test]
fn two_percent_of_names_carry_mixed_script() {
    let book = generate(1_000);
    let mixed = book.iter().filter(|row| !row.quote.name.is_ascii()).count();
    assert_eq!(mixed, 1_000 / MIXED_EVERY);
}

#[test]
fn numeric_sort_is_not_lexicographic() {
    let mut a = generate(2);
    a[0].quote.pe = 9.5;
    a[1].quote.pe = 10.25;
    assert_eq!(compare(&a[0], &a[1], FieldKind::Pe), Ordering::Less);
    assert_eq!(compare(&a[1], &a[0], FieldKind::Id), Ordering::Greater);
}

#[test]
fn tick_moves_one_stride_slice_per_step() {
    let mut prices = prices(&generate(400));
    let before = prices.clone();
    let moved = tick(&mut prices, 3);
    let changed = prices
        .iter()
        .zip(&before)
        .filter(|(new, old)| new.to_bits() != old.to_bits())
        .count();
    assert_eq!(moved, 10);
    assert_eq!(changed, 10);
    assert!(prices[3] < before[3]);
}

#[test]
fn price_must_be_finite_and_non_negative() {
    assert_eq!(parse_price(" 12.5 "), Some(12.5));
    assert_eq!(parse_price("0"), Some(0.0));
    assert_eq!(parse_price("-1"), None);
    assert_eq!(parse_price("nan"), None);
    assert_eq!(parse_price("abc"), None);
}

#[test]
fn filter_matches_symbol_or_name_case_insensitively() {
    let row = &generate(2)[1];
    assert!(row.matches(&row.quote.symbol.to_lowercase()));
    assert!(row.matches(&row.quote.name[..3]));
    assert!(!row.matches("no such ticker"));
}
