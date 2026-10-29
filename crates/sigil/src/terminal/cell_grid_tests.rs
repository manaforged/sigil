use super::*;

#[test]
fn a_wide_glyph_that_would_cross_the_edge_is_not_written() {
    let mut grid = CellGrid::new([0, 0, 3, 1], Cell::default());
    let written = grid.write_text([0, 0], "a日本", &Cell::default());
    assert_eq!(written, 3);
    assert_eq!(grid.row(0).map(<[Cell]>::len), Some(3));
    assert!(grid.get([3, 0]).is_none());
    assert!(grid.row(1).is_none());
}
