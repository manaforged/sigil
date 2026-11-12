use super::*;

#[test]
fn a_row_with_changes_far_apart_rewrites_only_the_changed_cells() {
    let caps = Capabilities::default();
    let row: Vec<Cell> = (0..40)
        .map(|index| Cell {
            text: if index % 2 == 0 { "x" } else { "y" }.into(),
            ..Cell::blank()
        })
        .collect();
    let mut prev = Grid::new(40, 1);
    prev.set_row(0, &row);
    let changed: Vec<Cell> = row
        .iter()
        .enumerate()
        .map(|(index, cell)| match index {
            2 => Cell {
                text: "A".into(),
                ..*cell
            },
            37 => Cell {
                text: "B".into(),
                ..*cell
            },
            _ => *cell,
        })
        .collect();
    let mut next = Grid::new(40, 1);
    next.set_row(0, &changed);
    let mut out = Vec::new();
    super::super::dirty(&mut out, &prev, &next, &caps, None).expect("write");
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains('A') && text.contains('B'), "{text:?}");
    assert!(
        !text.contains("xyxyxy"),
        "rewrote the unchanged middle: {text:?}"
    );
}

#[test]
fn a_row_limited_write_matches_the_full_write() {
    let caps = Capabilities::default();
    let base: Vec<Cell> = (0..20)
        .map(|index| Cell {
            text: if index % 2 == 0 {
                "x".into()
            } else {
                "y".into()
            },
            ..Cell::default()
        })
        .collect();
    let mut prev = Grid::new(20, 64);
    for row in 0..64 {
        prev.set_row(row, &base);
    }
    let mut next = prev.clone();
    let mut changed = base.clone();
    if let Some(cell) = changed.get_mut(7) {
        *cell = Cell {
            text: "Z".into(),
            ..Cell::default()
        };
    }
    next.set_row(41, &changed);
    let mut full = Vec::new();
    super::super::dirty(&mut full, &prev, &next, &caps, None).expect("write");
    let mut limited = Vec::new();
    let damage = super::super::diff::RowDamage::new(vec![41], false);
    super::super::dirty(&mut limited, &prev, &next, &caps, Some(&damage)).expect("write");
    assert_eq!(
        String::from_utf8_lossy(&limited),
        String::from_utf8_lossy(&full)
    );
}
