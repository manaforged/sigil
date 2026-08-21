use super::{EquivalenceCheck, book_held, tail_window};

#[test]
fn book_must_match_the_declared_count() {
    assert!(matches!(
        book_held(1_000_000, 1_000_000),
        EquivalenceCheck::Pass
    ));
    assert!(matches!(
        book_held(100_000, 1_000_000),
        EquivalenceCheck::Fail { .. }
    ));
    assert!(matches!(book_held(0, 0), EquivalenceCheck::Fail { .. }));
}

#[test]
fn tail_reaches_the_last_row() {
    assert!(matches!(
        tail_window(999_960, 40, 1_000_000),
        EquivalenceCheck::Pass
    ));
    assert!(matches!(
        tail_window(999_920, 80, 1_000_000),
        EquivalenceCheck::Pass
    ));
    assert!(matches!(
        tail_window(0, 41, 1_000_000),
        EquivalenceCheck::Fail { .. }
    ));
}
