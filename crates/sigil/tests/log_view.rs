use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::widgets::{Log, log_view};
use sigil_kittest::Harness;

const COLS: u16 = 30;
const ROWS: u16 = 6;

fn push_lines(log: Log, range: std::ops::Range<u32>) {
    for n in range {
        log.push(format!("line {n:03}"));
    }
}

fn view(log: Log) -> impl IntoNode {
    column()
        .layout(LayoutStyle::flex_column().fill())
        .child(log_view(log).fill())
}

#[test]
fn a_log_follows_the_tail_until_the_user_scrolls_up() {
    Owner::scope("log-view-follow", || {
        let log = Log::new(1_000);
        push_lines(log, 0..50);
        let mut ui = Harness::new(move || view(log)).size(COLS, ROWS);
        let screen = ui.settle().text();
        assert!(screen.contains("line 049"), "tail shown:\n{screen}");
        assert!(!screen.contains("line 000"), "head hidden:\n{screen}");

        ui.focus_next().press(NamedKey::Home).settle();
        push_lines(log, 50..51);
        let screen2 = ui.settle().text();
        assert!(
            screen2.contains("line 000"),
            "scrolled up stays put:\n{screen2}"
        );
        assert!(!screen2.contains("line 050"), "no jump to tail:\n{screen2}");

        ui.press(NamedKey::End).settle();
        push_lines(log, 51..52);
        let screen3 = ui.settle().text();
        assert!(
            screen3.contains("line 051"),
            "following resumes at the end:\n{screen3}"
        );
    });
}

#[test]
fn a_log_drops_its_oldest_lines_past_the_limit() {
    Owner::scope("log-view-cap", || {
        let log = Log::new(40);
        push_lines(log, 0..55);
        log.push("multi\nline");
        assert_eq!(log.len(), 40);
        let mut ui = Harness::new(move || view(log)).size(COLS, ROWS);
        let screen = ui.settle().text();
        assert!(
            screen.contains("line"),
            "last pushed text split into rows:\n{screen}"
        );
        ui.focus_next().press(NamedKey::Home).settle();
        let screen2 = ui.text();
        assert!(
            screen2.contains("line 017"),
            "oldest kept line first:\n{screen2}"
        );
        assert!(
            !screen2.contains("line 016"),
            "older lines dropped:\n{screen2}"
        );
        log.clear();
        assert!(log.is_empty());
    });
}

#[test]
fn a_trailing_line_break_adds_no_empty_line() {
    Owner::scope("log-newline", || {
        let log = Log::new(10);
        log.push("server ready\n");
        log.push("listening\r\n");
        assert_eq!(log.len(), 2);
        let mut ui = Harness::new(move || view(log)).size(COLS, ROWS);
        ui.settle().expect_text("listening");
    });
}

#[test]
fn pushing_empty_text_adds_one_empty_line() {
    Owner::scope("log-empty", || {
        let log = Log::new(10);
        log.push("first");
        log.push("");
        log.push("third");
        assert_eq!(log.len(), 3);
    });
}
