#![allow(clippy::expect_used)]

use sigil::testing::with_cx;

use crate::app_tree;
use crate::state::{Page, Shell, Where};

#[test]
fn tree_builds() {
    with_cx("workbench-tree", |cx| {
        let root = app_tree(cx);
        assert!(
            cx.first_child(root).is_some(),
            "root must have the rail and the main column",
        );
    });
}

#[test]
fn select_and_send() {
    with_cx("workbench-send", |cx| {
        let shell = Shell::seed();
        shell.select(1);
        assert_eq!(shell.selected.peek(), 1);
        shell.draft.set("hello".into());
        shell.send();
        let thread = shell.current().expect("thread 1");
        assert!(
            thread.lines.iter().any(|line| line.body == "hello"),
            "send appends the draft to the selected thread",
        );
        assert!(shell.draft.peek().is_empty());
        let _ = cx.add(crate::root(shell));
        shell.page.set(Page::Tools);
        assert_eq!(shell.page.peek(), Page::Tools);
    });
}

#[test]
fn new_thread_send_creates() {
    with_cx("workbench-new", |cx| {
        let shell = Shell::seed();
        shell.new_thread();
        assert_eq!(shell.selected.peek(), 0);
        shell.draft.set("Build snake".into());
        shell.send();
        assert_ne!(shell.selected.peek(), 0);
        let thread = shell.current().expect("created thread");
        assert_eq!(thread.title, "Build snake");
        assert_eq!(thread.lines.len(), 1);
        assert!(shell.draft.peek().is_empty());
        let _ = cx.add(crate::root(shell));
    });
}

#[test]
fn pages_and_diff_toggle() {
    with_cx("workbench-pages", |cx| {
        let shell = Shell::seed();
        shell.page.set(Page::Schedules);
        shell.toggle_day(0);
        assert!(shell.day_on(0));
        shell.auto_daily.set(false);
        assert!(!shell.auto_daily.peek());
        shell.run.set(Where::Cloud);
        assert_eq!(shell.run.peek(), Where::Cloud);
        shell.toggle_diff();
        assert!(shell.show_diff.peek());
        let _ = cx.add(crate::root(shell));
        shell.page.set(Page::Tools);
        let _ = cx.add(crate::root(shell));
    });
}
