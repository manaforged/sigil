#![allow(clippy::unwrap_used, clippy::float_cmp)]

use super::*;
use crate::table::columns::TableColumn;
use sigil_reactive::Runtime;

fn state_with(widths: &[(&'static str, ColWidth)]) -> TableState<u32> {
    let columns = widths
        .iter()
        .map(|&(id, width)| TableColumn::new(id, id).width(width))
        .collect();
    TableState::new(vec![1_u32, 2, 3], columns)
}

#[test]
fn fixed_columns_sum_to_extent() {
    Runtime::scope("hscroll-fixed-extent", || {
        let state = state_with(&[
            ("a", ColWidth::Fixed(150.0)),
            ("b", ColWidth::Fixed(150.0)),
            ("c", ColWidth::Fixed(150.0)),
        ]);
        let hs = bind(&state, Some(200.0), 0.0);
        assert_eq!(hs.extent.get(), 450.0);
        assert_eq!(hs.max.get(), 250.0, "extent minus viewport");
    });
}

#[test]
fn fraction_columns_count_their_floor() {
    Runtime::scope("hscroll-fraction-floor", || {
        // Mirrors flex_cell: once Fixed(300) overflows a 400 px
        // viewport, the fraction column squeezes to its min width.
        let state = TableState::new(
            vec![1_u32, 2, 3],
            vec![
                TableColumn::new("a", "a").width(ColWidth::Fixed(300.0)),
                TableColumn::new("b", "b")
                    .width(ColWidth::Fraction(1.0))
                    .min_width(192.0),
            ],
        );
        let hs = bind(&state, Some(400.0), 0.0);
        assert_eq!(hs.extent.get(), 492.0);
        assert_eq!(hs.max.get(), 92.0);
    });
}

#[test]
fn fitting_content_has_zero_max() {
    Runtime::scope("hscroll-fits", || {
        let state = state_with(&[("a", ColWidth::Fixed(100.0)), ("b", ColWidth::Fixed(100.0))]);
        let hs = bind(&state, Some(200.0), 0.0);
        assert_eq!(hs.extent.get(), 200.0);
        assert_eq!(hs.max.get(), 0.0, "no overflow, no scroll range");
    });
}

#[test]
fn resize_override_widens_extent_reactively() {
    Runtime::scope("hscroll-resize", || {
        let state = state_with(&[("a", ColWidth::Fixed(100.0)), ("b", ColWidth::Fixed(100.0))]);
        let hs = bind(&state, Some(200.0), 0.0);
        assert_eq!(hs.max.get(), 0.0);
        state.pin_column_width("a", 500.0);
        assert_eq!(hs.extent.get(), 600.0);
        assert_eq!(hs.max.get(), 400.0, "a pinned column can force overflow");
    });
}

#[test]
fn hidden_column_shrinks_extent_reactively() {
    Runtime::scope("hscroll-hide", || {
        let state = state_with(&[
            ("a", ColWidth::Fixed(150.0)),
            ("b", ColWidth::Fixed(150.0)),
            ("c", ColWidth::Fixed(150.0)),
        ]);
        let hs = bind(&state, Some(200.0), 0.0);
        assert_eq!(hs.max.get(), 250.0);
        state.hide_column("c");
        assert_eq!(hs.extent.get(), 300.0);
        assert_eq!(hs.max.get(), 100.0);
    });
}

#[test]
fn measured_viewport_follows_the_probe() {
    Runtime::scope("hscroll-probe", || {
        let state = state_with(&[
            ("a", ColWidth::Fixed(150.0)),
            ("b", ColWidth::Fixed(150.0)),
            ("c", ColWidth::Fixed(150.0)),
        ]);
        let hs = bind(&state, None, 0.0);
        let probe = hs.probe.expect("no explicit width installs a probe");
        assert_eq!(hs.viewport.get(), 0.0, "unmeasured until layout publishes");
        probe.set((200.0, 480.0));
        assert_eq!(hs.viewport.get(), 200.0);
        assert_eq!(hs.max.get(), 250.0);
    });
}

#[test]
fn select_column_prefix_counts_as_fixed_pixels() {
    Runtime::scope("hscroll-select-prefix", || {
        // The opt-in checkbox column prepends its fixed width to the
        // extent: 2 × Fixed(100) + 36 px prefix overflows a 200 px
        // viewport by exactly the prefix.
        let state = state_with(&[("a", ColWidth::Fixed(100.0)), ("b", ColWidth::Fixed(100.0))]);
        let hs = bind(&state, Some(200.0), 36.0);
        assert_eq!(hs.extent.get(), 236.0);
        assert_eq!(hs.max.get(), 36.0);
    });
}

#[test]
fn explicit_viewport_installs_no_probe() {
    Runtime::scope("hscroll-explicit", || {
        let state = state_with(&[("a", ColWidth::Fixed(100.0))]);
        let hs = bind(&state, Some(200.0), 0.0);
        assert!(hs.probe.is_none());
        assert_eq!(hs.viewport.get(), 200.0);
    });
}
