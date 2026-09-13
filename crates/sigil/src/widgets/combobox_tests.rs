#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use super::*;
use crate::kernel::Cx;
use crate::reactive::Runtime;

fn options() -> Vec<String> {
    [
        "Accounts",
        "Account History",
        "Profile",
        "Proxy Group",
        "Proxy Rules",
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

fn labels(matches: &[(usize, String)]) -> Vec<&str> {
    matches.iter().map(|(_, label)| label.as_str()).collect()
}

#[test]
fn filter_matches_case_insensitive_substring() {
    Runtime::scope("combobox-filter", || {
        let state = Combobox::new(options());
        state.edit_query("aCC");
        assert_eq!(labels(&state.filtered()), ["Accounts", "Account History"]);
        state.edit_query("proxy");
        assert_eq!(labels(&state.filtered()), ["Proxy Group", "Proxy Rules"]);
    });
}

#[test]
fn empty_query_matches_every_option() {
    Runtime::scope("combobox-filter-empty", || {
        let state = Combobox::new(options());
        assert_eq!(state.filtered().len(), 5);
    });
}

#[test]
fn editing_opens_popup_and_resets_cursor() {
    Runtime::scope("combobox-edit-opens", || {
        let state = Combobox::new(options());
        state.move_cursor(2);
        state.close();
        state.edit_query("pro");
        assert!(state.is_open(), "editing opens the popup");
        assert_eq!(state.cursor(), 0, "editing resets the cursor");
    });
}

#[test]
fn clearing_text_clears_selection() {
    Runtime::scope("combobox-clear", || {
        let state = Combobox::new(options());
        state.commit(2);
        assert_eq!(state.selected(), Some(2));
        state.edit_query("");
        assert_eq!(state.selected(), None, "empty text clears the selection");
        state.commit(2);
        state.edit_query("ProfX");
        assert_eq!(
            state.selected(),
            None,
            "text diverging from the label clears the selection"
        );
    });
}

#[test]
fn retyping_exact_label_keeps_selection() {
    Runtime::scope("combobox-retype", || {
        let state = Combobox::new(options());
        state.commit(2);
        state.edit_query("Profile");
        assert_eq!(state.selected(), Some(2));
    });
}

#[test]
fn commit_writes_value_closes_and_shows_label() {
    Runtime::scope("combobox-commit", || {
        let state = Combobox::new(options());
        state.edit_query("prox");
        state.commit(3);
        assert_eq!(state.selected(), Some(3));
        assert_eq!(state.selected_option().as_deref(), Some("Proxy Group"));
        assert_eq!(state.query(), "Proxy Group", "field shows the label");
        assert!(!state.is_open(), "popup closes on commit");
    });
}

#[test]
fn cursor_move_clamps_to_filtered_bounds() {
    Runtime::scope("combobox-cursor-clamp", || {
        let state = Combobox::new(options());
        state.edit_query("proxy");
        for _ in 0..10 {
            state.move_cursor(1);
        }
        assert_eq!(state.cursor(), 1, "clamped at last filtered row");
        for _ in 0..10 {
            state.move_cursor(-1);
        }
        assert_eq!(state.cursor(), 0, "clamped at first row");
    });
}

#[test]
fn commit_cursor_commits_highlighted_row() {
    Runtime::scope("combobox-commit-cursor", || {
        let state = Combobox::new(options());
        state.edit_query("proxy");
        state.move_cursor(1);
        assert!(state.commit_cursor());
        assert_eq!(state.selected_option().as_deref(), Some("Proxy Rules"));
    });
}

#[test]
fn commit_cursor_with_no_matches_is_noop() {
    Runtime::scope("combobox-commit-empty", || {
        let state = Combobox::new(options());
        state.edit_query("zzz");
        assert!(!state.commit_cursor());
        assert_eq!(state.selected(), None);
        assert!(state.is_open(), "failed commit leaves the popup open");
    });
}

#[test]
fn open_places_cursor_on_selected_row() {
    Runtime::scope("combobox-open-cursor", || {
        let state = Combobox::new(options());
        state.commit(1);
        state.open();
        let under_cursor = state.filtered().get(state.cursor()).map(|(i, _)| *i);
        assert_eq!(
            under_cursor,
            state.selected(),
            "cursor lands on the selected option's filtered row"
        );
    });
}

#[test]
fn widget_shows_popup_only_when_open_with_matches() {
    Runtime::scope("combobox-build", || {
        let state = Combobox::new(options());
        let mut cx = Cx::new_root();
        let root = cx.add(combobox(state));
        cx.flush_reactive_children();
        let popup_host = cx.nth_child(root, 1).expect("popup host child");
        assert_eq!(cx.child_count(popup_host), 0, "closed: no popup");
        state.open();
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(popup_host), 1, "open: popup rendered");
        state.edit_query("zzz");
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(popup_host), 0, "no matches: popup hidden");
        state.close();
        state.edit_query("proxy");
        state.close();
        cx.flush_reactive_children();
        assert_eq!(cx.child_count(popup_host), 0, "closed again: popup hidden");
    });
}
