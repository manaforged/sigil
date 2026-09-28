use super::*;

#[test]
fn off_thread_print_reports_not_live() {
    let err = print_above("line").expect_err("no live terminal on this thread");
    assert_eq!(
        err.get_ref()
            .and_then(|inner| inner.downcast_ref::<PrintAboveError>()),
        Some(&PrintAboveError::NotLive)
    );
}

#[test]
fn escape_sequences_never_reach_the_terminal() {
    assert_eq!(strip("\x1b[31mred\x1b[0m\x1b]0;title\x07!\u{7}"), "red!");
}

#[test]
fn wide_lines_clip_to_the_column_count() {
    assert_eq!(clip("abcdef", 3), "abc");
    assert_eq!(clip("日本語", 5), "日本");
}
