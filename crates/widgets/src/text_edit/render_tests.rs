use super::{MAX_LINES, caret_line_col, truncation_notice};

#[test]
fn no_notice_at_or_under_the_cap() {
    assert!(truncation_notice(0).is_none());
    assert!(truncation_notice(MAX_LINES).is_none());
}

#[test]
fn notice_counts_the_withheld_lines() {
    let notice = truncation_notice(MAX_LINES + 3).expect("notice past the cap");
    assert!(
        notice.contains('3') && notice.contains("more line"),
        "truncation must be loud and name the count, got {notice:?}",
    );
}

#[test]
fn caret_line_col_maps_offsets_across_lines() {
    let buf = "ab\ncde\nf";
    assert_eq!(caret_line_col(buf, 0), (0, 0), "start of line 0");
    assert_eq!(caret_line_col(buf, 2), (0, 2), "end of line 0 (before \\n)");
    assert_eq!(
        caret_line_col(buf, 3),
        (1, 0),
        "start of line 1 (after \\n)"
    );
    assert_eq!(caret_line_col(buf, 5), (1, 2), "middle of line 1");
    assert_eq!(caret_line_col(buf, 7), (2, 0), "start of line 2");
    assert_eq!(caret_line_col(buf, 8), (2, 1), "end of buffer");
}

#[test]
fn caret_line_col_clamps_past_the_end() {
    assert_eq!(caret_line_col("hi", 99), (0, 2), "clamped to buffer length");
}

#[test]
fn line_start_byte_sums_prior_lines_and_newlines() {
    use super::line_start_byte;
    let buf = "ab
cde
f";
    assert_eq!(line_start_byte(buf, 0), 0);
    assert_eq!(line_start_byte(buf, 1), 3, "after 'ab\n'");
    assert_eq!(line_start_byte(buf, 2), 7, "after 'ab\ncde\n'");
    assert_eq!(line_start_byte(buf, 99), buf.len(), "clamped past the end");
}
