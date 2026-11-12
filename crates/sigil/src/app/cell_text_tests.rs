use super::{Ambiguous, ambiguous, cell_width, ellipsize_cells, ellipsize_cells_start, fit_cells};

#[test]
fn ambiguous_wide_doubles_box_drawing() {
    ambiguous(Ambiguous::Narrow);
    assert_eq!(cell_width("─·°"), 3);
    ambiguous(Ambiguous::Wide);
    assert_eq!(cell_width("─·°"), 6);
    ambiguous(Ambiguous::Narrow);
}

#[test]
fn ellipsis_keeps_wide_text_inside_its_cells() {
    assert_eq!(ellipsize_cells("日本語テキスト", 5), "日本…");
    assert_eq!(ellipsize_cells_start("abcdef", 4), "…def");
    assert_eq!(fit_cells("ab", 4), "ab  ");
}
