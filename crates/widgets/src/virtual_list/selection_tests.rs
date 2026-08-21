#![allow(clippy::indexing_slicing, missing_docs)]

use super::*;

#[test]
fn selection_single_replaces_on_click() {
    let mut state = SelectionState::new(SelectionMode::Single);

    state.click(3);
    state.click(5);

    assert_eq!(selected(&state), vec![5]);
}

#[test]
fn selection_multi_toggles_on_ctrl_click() {
    let mut state = SelectionState::new(SelectionMode::Multi);

    state.ctrl_click(3);
    state.ctrl_click(5);
    state.ctrl_click(3);

    assert_eq!(selected(&state), vec![5]);
}

#[test]
fn selection_extend_ranges_on_shift_click() {
    let mut state = SelectionState::new(SelectionMode::Extend);

    state.click(2);
    state.shift_click(5);

    assert_eq!(selected(&state), vec![2, 3, 4, 5]);
}

#[test]
fn selection_none_ignores_clicks() {
    let mut state = SelectionState::new(SelectionMode::None);

    state.click(1);

    assert!(selected(&state).is_empty());
}

fn selected(state: &SelectionState) -> Vec<usize> {
    state.selected_indices().collect()
}
