#![allow(clippy::unwrap_used, clippy::float_cmp)]

use super::*;
use crate::reactive::Runtime;
use crate::widgets::table::columns::TableColumn;

fn fixed(ids: &[&'static str], px: f32) -> TableState<u32> {
    let columns = ids
        .iter()
        .map(|&id| TableColumn::new(id, id).width(ColWidth::Fixed(px)))
        .collect();
    TableState::new(vec![1_u32, 2, 3], columns)
}

#[test]
fn fixed_columns_sum_to_extent_and_hiding_shrinks_it() {
    Runtime::scope("hscroll-fixed-extent", || {
        let state = fixed(&["a", "b", "c"], 150.0);
        let hs = bind(&state, Some(200.0), 0.0);
        assert!(hs.probe.is_none());
        assert_eq!(hs.viewport.get(), 200.0);
        assert_eq!(hs.extent.get(), 450.0);
        assert_eq!(hs.max.get(), 250.0, "extent minus viewport");
        state.hide_column("c");
        assert_eq!(hs.extent.get(), 300.0);
        assert_eq!(hs.max.get(), 100.0);
    });
}

#[test]
fn fraction_columns_count_their_floor() {
    Runtime::scope("hscroll-fraction-floor", || {
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
fn fitting_content_scrolls_only_after_a_resize_or_select_prefix() {
    Runtime::scope("hscroll-fits", || {
        let state = fixed(&["a", "b"], 100.0);
        let hs = bind(&state, Some(200.0), 0.0);
        assert_eq!(hs.extent.get(), 200.0);
        assert_eq!(hs.max.get(), 0.0, "no overflow, no scroll range");
        let prefixed = bind(&state, Some(200.0), 36.0);
        assert_eq!(prefixed.extent.get(), 236.0);
        assert_eq!(prefixed.max.get(), 36.0);
        state.pin_column_width("a", 500.0);
        assert_eq!(hs.extent.get(), 600.0);
        assert_eq!(hs.max.get(), 400.0, "a pinned column can force overflow");
    });
}

#[test]
fn measured_viewport_follows_the_probe() {
    Runtime::scope("hscroll-probe", || {
        let hs = bind(&fixed(&["a", "b", "c"], 150.0), None, 0.0);
        let probe = hs.probe.expect("no explicit width installs a probe");
        assert_eq!(hs.viewport.get(), 0.0, "unmeasured until layout publishes");
        probe.set((200.0, 480.0));
        assert_eq!(hs.viewport.get(), 200.0);
        assert_eq!(hs.max.get(), 250.0);
    });
}
