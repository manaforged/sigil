#![allow(clippy::indexing_slicing)]

use super::*;

#[test]
fn selection_single_replaces_on_click() {
    let mut state = Selection::new(SelectionMode::Single);

    state.select(3);
    state.select(5);

    assert_eq!(selected(&state), vec![5]);
}

#[test]
fn selection_multi_toggles_on_ctrl_click() {
    let mut state = Selection::new(SelectionMode::Multi);

    state.toggle(3);
    state.toggle(5);
    state.toggle(3);

    assert_eq!(selected(&state), vec![5]);
}

#[test]
fn selection_extend_ranges_on_shift_click() {
    let mut state = Selection::new(SelectionMode::Extend);

    state.select(2);
    state.extend(5);

    assert_eq!(selected(&state), vec![2, 3, 4, 5]);
}

#[test]
fn selection_none_ignores_clicks() {
    let mut state = Selection::new(SelectionMode::None);

    state.select(1);

    assert!(selected(&state).is_empty());
}

fn selected(state: &Selection) -> Vec<usize> {
    state.indices().collect()
}
