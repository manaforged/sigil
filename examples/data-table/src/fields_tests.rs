use super::*;
use crate::quote::quote_at;

#[test]
fn fields_match_gpui_width_sum() {
    assert_eq!(FIELDS.len(), 24);
    let width: f32 = FIELDS.iter().map(|field| field.width).sum();
    assert!((width - 2026.0).abs() < f32::EPSILON);
}

#[test]
fn header_label_uppercases_underscores() {
    assert_eq!(header_label("last_done"), "LAST DONE");
}

#[test]
fn id_cell_is_the_row_index() {
    let quote = quote_at(7);
    assert_eq!(cell_text(7, &quote, FieldKind::Id), "7");
}
