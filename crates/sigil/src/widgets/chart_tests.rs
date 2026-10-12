#![allow(clippy::unwrap_used, clippy::float_cmp, clippy::shadow_unrelated)]

use super::*;
use crate::Host;
use crate::kernel::{IntoNode, container};
use crate::reactive::Runtime;

#[test]
fn nice_max_steps_are_1_2_5() {
    assert_eq!(nice_max(3.0), 5.0);
    assert_eq!(nice_max(12.0), 20.0);
    assert_eq!(nice_max(0.0), 1.0);
}

#[test]
fn empty_and_line_charts_build() {
    Runtime::scope("chart-empty", || {
        let mut cx = Cx::new_root();
        let id = chart([]).build(&mut cx);
        assert!(cx.get(id).is_some());
        let id = chart([1.0, 4.0, 2.0, 8.0]).line().build(&mut cx);
        assert!(cx.get(id).is_some());
    });
}

#[test]
fn bar_chart_builds() {
    Runtime::scope("chart-bar", || {
        let mut host = Host::new();
        host.mount(move || {
            container().child(chart([4.0, 8.0, 3.0, 10.0]).labels(["Mon", "Tue", "Wed", "Thu"]))
        });
        host.layout((320.0, 200.0)).unwrap();
        let scene = host.scene((320.0, 200.0)).unwrap();
        assert!(scene.find_text("Mon").is_some());
    });
}

#[test]
fn narrow_chart_dates_keep_the_day_digits_visible() {
    Runtime::scope("chart-compact-dates", || {
        let mut host = Host::new();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column())
                .child(chart([1.0; 7]).width(300.0).labels([
                    "Sep 20", "Sep 21", "Sep 22", "Sep 23", "Sep 24", "Sep 25", "Sep 26",
                ]))
                .child(text("26").nowrap().class("text-xs"))
        });
        host.layout((400.0, 300.0)).unwrap();
        let scene = host.scene((400.0, 300.0)).unwrap();
        let reference = scene.find_text("26").unwrap().run.positioned_glyphs();
        let date = scene.find_text("Sep 26").unwrap().run.positioned_glyphs();
        let digits: Vec<_> = reference.iter().map(|glyph| glyph.glyph_id).collect();
        let date_glyphs: Vec<_> = date.iter().map(|glyph| glyph.glyph_id).collect();
        assert!(!digits.is_empty(), "reference digits are rendered");
        assert!(
            date_glyphs.ends_with(&digits),
            "chart must paint day digits, not an ellipsis: {date_glyphs:?}"
        );
    });
}

#[test]
fn a_whole_number_chart_never_labels_a_fraction() {
    let labelled = |whole: bool| {
        Runtime::scope("chart-whole-ticks", move || {
            let mut host = Host::new();
            host.mount(move || {
                let plot = chart([0.0, 0.0, 1.0]).bar().height(400.0);
                container()
                    .layout(LayoutStyle::flex_column())
                    .child(if whole { plot.whole_numbers() } else { plot })
            });
            host.layout((600.0, 500.0)).unwrap();
            host.scene((600.0, 500.0))
                .unwrap()
                .find_text("0.5")
                .is_some()
        })
    };
    assert!(
        labelled(false),
        "a fractional midpoint is labelled by default"
    );
    assert!(
        !labelled(true),
        "a whole-number chart skips a fractional tick"
    );
}
