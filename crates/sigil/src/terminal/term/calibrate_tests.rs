use super::{Measured, calibrate, keep, measure};
use crate::app::{CellWidth, cell_width, set_cell_width};

#[test]
fn a_partial_reply_measures_nothing() {
    assert_eq!(measure(&[(2, 1), (2, 3)]), Measured::Silent);
}

#[test]
fn wide_modifiers_switch_to_measured_scalar_widths() -> std::io::Result<()> {
    keep(Measured::Columns([4, 1, 1, 2]));
    calibrate(&mut Vec::new())?;
    assert_eq!(cell_width("\u{200d}"), 1);
    assert_eq!(cell_width("1\u{fe0f}\u{20e3}"), 2);
    set_cell_width(CellWidth::Unicode);
    Ok(())
}

#[test]
fn a_modifier_as_wide_as_unicode_keeps_unicode_widths() -> std::io::Result<()> {
    keep(Measured::Columns([2, 1, 1, 2]));
    calibrate(&mut Vec::new())?;
    assert_eq!(cell_width("\u{200d}"), 0);
    Ok(())
}
