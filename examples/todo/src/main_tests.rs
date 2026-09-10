#![allow(clippy::unwrap_used)]

use super::*;
use sigil::dialog::mock;
use sigil::testing::with_cx;

#[test]
fn tree_builds_without_panic() {
    with_cx("todo-smoke", |cx| {
        let state = TodoState::seed();
        let root = cx.add(app_tree(&state));
        assert!(cx.child_count(root) > 0, "root must have children");
    });
}

#[test]
fn open_via_mocked_dialog_loads_tasks() {
    with_cx("todo-open", |_cx| {
        let path = std::env::temp_dir().join("sigil-todo-open-test.txt");
        std::fs::write(&path, "[x] shipped\n[ ] pending\n").unwrap();
        mock::push_path(Some(path.clone()));

        let state = TodoState::seed();
        if let Some(picked) = FileDialog::new().open_file() {
            state.load_from(&picked);
        }

        assert_eq!(state.todos.peek().len(), 2, "both tasks loaded");
        drop(std::fs::remove_file(&path));
    });
}

#[test]
fn on_menu_new_clears_the_list() {
    with_cx("todo-menu-new", |_| {
        let state = TodoState::seed();
        assert!(!state.todos.peek().is_empty());
        on_menu(state, "new");
        assert!(
            state.todos.peek().is_empty(),
            "File → New List must empty the tasks"
        );
    });
}
