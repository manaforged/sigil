#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use super::*;

#[test]
fn script_touches_every_interaction_once() {
    let steps: Vec<Step> = (0..PULSES).filter_map(step).collect();
    let count = |f: fn(&Step) -> bool| steps.iter().filter(|s| f(s)).count();
    assert_eq!(count(|s| matches!(s, Step::Scroll(_))), 450);
    assert_eq!(count(|s| matches!(s, Step::Sort(_))), 2);
    assert_eq!(count(|s| matches!(s, Step::Filter(_))), 2);
    assert_eq!(count(|s| matches!(s, Step::Commit(_))), EDITS as usize);
    assert_eq!(count(|s| matches!(s, Step::Undo)), EDITS as usize);
    assert_eq!(count(|s| matches!(s, Step::Theme)), 2);
    assert_eq!(step(PULSES - 1), None);
}

#[test]
fn edits_finish_before_the_theme_toggle() {
    let last_undo = (0..PULSES)
        .rev()
        .find(|&p| step(p) == Some(Step::Undo))
        .unwrap();
    assert!(last_undo < 860, "{last_undo}");
}

#[test]
fn expectation_matches_a_replayed_book() {
    let book = data_table::ledger::generate(400);
    let want = Tally::expect(&book);
    assert_eq!(want.scroll_rows, 1800);
    assert_eq!(want.moved, moved_total(400, PULSES));
    assert!(want.filter_hits > 0);
    let mut got = want.clone();
    assert!(want.checks(&got).iter().all(|c| c.pass));
    got.undo_depth = 1;
    let failed: Vec<_> = want.checks(&got).into_iter().filter(|c| !c.pass).collect();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].name, "undo_depth");
}
