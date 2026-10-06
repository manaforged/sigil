use sigil::prelude::*;
use sigil::style::{NumericAtomKind, Transform2D};

use super::present;

fn column_of(grid: &str, needle: &str) -> usize {
    grid.lines()
        .find_map(|line| line.find(needle).map(|at| line[..at].chars().count()))
        .unwrap_or_else(|| panic!("no {needle:?} in:\n{grid}"))
}

#[test]
fn a_translated_subtree_paints_at_its_moved_place() {
    let place = |shift: f32| {
        present(
            move || {
                container()
                    .layout(LayoutStyle::block().width(160.0).height(24.0))
                    .child(
                        container()
                            .transform(Transform2D::IDENTITY.origin(0.0, 0.0))
                            .animate(NumericAtomKind::TranslateX, signal(shift))
                            .child(text("moved")),
                    )
            },
            30,
            4,
            &[],
        )
    };
    let still = column_of(&place(0.0), "moved");
    let shifted = column_of(&place(48.0), "moved");
    assert_eq!(shifted, still + 6, "48 px is six 8 px cells");
}
