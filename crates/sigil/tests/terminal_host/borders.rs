use super::*;
use sigil::prelude::Radius;

fn boxed(rounded: bool) -> String {
    present(
        move || {
            let frame = container()
                .layout(sigil::layout::LayoutStyle::block().padding_x(8.0))
                .border_w(1)
                .child(text("inside"));
            if rounded {
                frame.rounded(Radius::Md)
            } else {
                frame
            }
        },
        30,
        8,
        &[],
    )
}

#[test]
fn a_rounded_border_draws_rounded_corners_in_cells() {
    let rounded = boxed(true);
    assert!(rounded.contains("inside"), "{rounded}");
    for corner in ['╭', '╮', '╰', '╯'] {
        assert!(rounded.contains(corner), "missing {corner} in:\n{rounded}");
    }
    let square = boxed(false);
    assert!(square.contains('┌') && !square.contains('╭'), "{square}");
}

fn backed_rows(parent: sigil::layout::LayoutStyle) -> usize {
    let grid = painted(
        move || {
            container()
                .layout(parent.clone())
                .child(
                    container()
                        .layout(
                            sigil::layout::LayoutStyle::block()
                                .position(sigil::layout::Position::Absolute)
                                .inset(0.0),
                        )
                        .bg(sigil::prelude::Token::Accent),
                )
                .child(text("one"))
                .child(text("two"))
                .child(text("three"))
        },
        30,
        12,
    );
    (0..grid.rows())
        .filter(|&row| {
            grid.row(row)
                .get(1)
                .is_some_and(|cell| cell.bg != sigil::terminal::Ink::Bare)
        })
        .count()
}

#[test]
fn an_absolute_backdrop_stretches_to_an_auto_height_parent() {
    assert_eq!(
        backed_rows(sigil::layout::LayoutStyle::block()),
        3,
        "block parent"
    );
    assert_eq!(
        backed_rows(sigil::layout::LayoutStyle::flex_column()),
        3,
        "flex parent"
    );
}
