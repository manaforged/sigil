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
