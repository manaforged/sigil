use super::*;

fn width(atoms: &[StyleAtom]) -> Option<i16> {
    atoms.iter().find_map(|atom| match atom {
        StyleAtom::Width(Dimension::Px(Px(width))) => Some(*width),
        _ => None,
    })
}

#[test]
fn a_resting_thumb_fills_its_cell_on_a_grid_and_stays_thin_in_a_window() {
    crate::style::grid(0.0);
    assert_eq!(
        width(thumb_atoms(48, false)),
        Some(to_i16(THUMB_REST_WIDTH))
    );
    crate::style::grid(8.0);
    assert_eq!(width(thumb_atoms(48, false)), Some(to_i16(SCROLLBAR_WIDTH)));
    crate::style::grid(0.0);
}
