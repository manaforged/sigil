use super::{SparkMark, spark_glyph, sparkline};
use crate::style::Tone;

#[test]
fn a_fixed_range_maps_the_floor_and_the_ceiling_to_the_end_glyphs() {
    let line = sparkline([0.0, 50.0, 100.0]).range(0.0, 100.0);
    assert_eq!(line.glyphs(), "▁▄█");
}

#[test]
fn values_outside_the_range_clamp_to_the_end_glyphs() {
    assert_eq!(spark_glyph(-1.0), '▁');
    assert_eq!(spark_glyph(2.0), '█');
}

#[test]
fn a_two_row_block_graph_fills_columns_from_the_bottom() {
    let line = sparkline([0.0, 0.5, 1.0]).range(0.0, 1.0).rows(2);
    assert_eq!(line.graph(), vec!["  █".to_owned(), " ██".to_owned()]);
}

#[test]
fn a_one_row_braille_graph_sets_the_right_column_for_a_full_value() {
    let line = sparkline([0.0, 1.0])
        .range(0.0, 1.0)
        .mark(SparkMark::Braille);
    assert_eq!(line.graph(), vec!["\u{28B8}".to_owned()]);
}

#[test]
fn adjacent_cells_of_one_tone_share_a_run() {
    let line = sparkline([1.0, 1.0, 1.0]).range(0.0, 1.0).tones([
        Some(Tone::Warn),
        Some(Tone::Warn),
        None,
    ]);
    assert_eq!(
        line.runs(),
        vec![vec![
            (Tone::Warn, "██".to_owned()),
            (Tone::Primary, "█".to_owned()),
        ]]
    );
}
