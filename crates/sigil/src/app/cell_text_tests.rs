use super::{Ambiguous, cell_width, set_ambiguous};

#[test]
fn ambiguous_wide_doubles_box_drawing() {
    set_ambiguous(Ambiguous::Narrow);
    assert_eq!(cell_width("─·°"), 3);
    set_ambiguous(Ambiguous::Wide);
    assert_eq!(cell_width("─·°"), 6);
    set_ambiguous(Ambiguous::Narrow);
}
