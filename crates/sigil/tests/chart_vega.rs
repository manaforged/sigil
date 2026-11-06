#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::widgets::{ChartBuilder, chart};
use sigil_kittest::{Harness, Query};

fn fence(json: &str) -> String {
    format!("```vega-lite\n{json}\n```")
}

fn x_of(ui: &mut Harness, label: &str) -> f32 {
    ui.get(&Query::text(label))
        .bounds()
        .expect("bounds")
        .origin
        .x
}

#[test]
fn vega_temporal_months_keep_time_order() {
    let json = r#"{"data":{"values":[{"t":"Jan 1, 2024","v":1},{"t":"Feb 1, 2024","v":2},{"t":"Mar 1, 2024","v":3}]},"mark":"line","encoding":{"x":{"field":"t","type":"temporal"},"y":{"field":"v","type":"quantitative"}}}"#;
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).pixels(800, 400);
    ui.settle();
    let (jan, feb) = (x_of(&mut ui, "Jan 1, 2024"), x_of(&mut ui, "Feb 1, 2024"));
    assert!(jan < feb, "Jan at {jan} is right of Feb at {feb}");
}

#[test]
fn vega_area_with_color_stacks() {
    let json = r#"{"data":{"values":[{"x":"p","g":"A","y":6},{"x":"q","g":"A","y":6},{"x":"p","g":"B","y":6},{"x":"q","g":"B","y":6}]},"mark":"area","encoding":{"x":{"field":"x","type":"nominal"},"y":{"field":"y","type":"quantitative"},"color":{"field":"g","type":"nominal"}}}"#;
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).pixels(800, 400);
    ui.settle();
    assert!(
        ui.query(&Query::text("20")).is_some(),
        "y axis does not reach the stacked total 12: {}",
        ui.text()
    );
}

#[test]
fn vega_quantitative_x_line_is_spaced_by_value() {
    let json = r#"{"data":{"values":[{"x":1,"y":1},{"x":2,"y":2},{"x":10,"y":3}]},"mark":"line","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#;
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).pixels(800, 400);
    ui.settle();
    let (a, b, c) = (x_of(&mut ui, "1"), x_of(&mut ui, "2"), x_of(&mut ui, "10"));
    assert!(
        (b - a) * 3.0 < c - b,
        "x=1 at {a}, 2 at {b}, 10 at {c}: equal spacing"
    );
}

#[test]
fn scatter_of_one_large_point_has_distinct_ticks() {
    Owner::scope("scatter-big", || {
        let mut ui = Harness::new(|| {
            ChartBuilder::scatter(vec![(1e8_f32, 1e8_f32)])
                .tooltip()
                .width(400.0)
                .height(300.0)
        })
        .pixels(600, 400);
        ui.settle();
        let shown = ui.text();
        assert!(
            shown.matches("100000000").count() < 6,
            "every tick reads 100000000: {shown}"
        );
    });
}

#[test]
fn small_fractional_bars_keep_their_values() {
    let json = r#"{"data":{"values":[{"k":"a","v":0.4},{"k":"b","v":0.2}]},"mark":"bar","encoding":{"x":{"field":"k","type":"nominal"},"y":{"field":"v","type":"quantitative"}}}"#;
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).pixels(600, 400);
    ui.settle();
    let shown = ui.text();
    assert!(
        shown.contains("Name=\"0.4\""),
        "bar caption for 0.4: {shown}"
    );
}

#[test]
fn tiny_pie_values_are_not_zero() {
    Owner::scope("pie-tiny", || {
        let mut ui = Harness::new(|| {
            chart(vec![0.001_f32, 0.003])
                .pie()
                .labels(["A", "B"])
                .width(400.0)
                .height(200.0)
        })
        .pixels(600, 400);
        ui.settle();
        let shown = ui.text();
        assert!(!shown.contains("Value=\"0 · 25%\""), "pie legend: {shown}");
    });
}

#[test]
fn pie_of_huge_values_reads_fifty_percent() {
    Owner::scope("pie-huge", || {
        let mut ui = Harness::new(|| {
            chart(vec![3e38_f32, 3e38])
                .pie()
                .labels(["A", "B"])
                .width(400.0)
                .height(200.0)
        })
        .pixels(600, 400);
        ui.settle();
        let shown = ui.text();
        assert!(shown.contains("50%"), "pie legend: {shown}");
    });
}

#[test]
fn vega_sum_overflow_keeps_the_bar() {
    let json = r#"{"data":{"values":[{"k":"a","v":2e38},{"k":"a","v":2e38},{"k":"b","v":1}]},"mark":"bar","encoding":{"x":{"field":"k","type":"nominal"},"y":{"field":"v","type":"quantitative","aggregate":"sum"}}}"#;
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).pixels(600, 400);
    ui.settle();
    let shown = ui.text();
    assert!(!shown.contains("inf"), "axis or caption reads inf: {shown}");
}
