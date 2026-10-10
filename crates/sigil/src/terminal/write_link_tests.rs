use super::super::{Cell, Grid, Pen, row_bytes};
use crate::terminal::caps::Caps;

#[test]
fn a_link_with_a_control_character_never_reaches_the_terminal() {
    let caps = Caps {
        hyperlinks: true,
        ..Caps::default()
    };
    let mut grid = Grid::new(4, 1);
    let cell = |url: &str| Cell {
        text: "x".into(),
        link: Some(url.into()),
        ..Cell::blank()
    };
    grid.set_row(
        0,
        &[
            cell("https://a.test/\u{1b}\\\u{1b}]52;c;cHduZWQ=\u{7}"),
            cell("https://b.test/"),
        ],
    );
    let mut out: Vec<u8> = Vec::new();
    row_bytes(&mut out, &grid, 0, &mut Pen::default(), &caps).expect("write");
    let out = String::from_utf8(out).expect("utf8");
    assert!(!out.contains("]52"), "clipboard write escaped: {out:?}");
    assert!(!out.contains("a.test"), "unsafe link written: {out:?}");
    assert!(
        out.contains("\u{1b}]8;;https://b.test/\u{1b}\\"),
        "safe link lost: {out:?}"
    );
}
