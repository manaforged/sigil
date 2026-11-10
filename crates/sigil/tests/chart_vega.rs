#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test fails loudly on a missing value"
)]

use sigil::geometry::Rect;
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::widgets::{ChartBuilder, chart};
use sigil_kittest::{Harness, Query};

fn fence(json: &str) -> String {
    format!("```vega-lite\n{json}\n```")
}

fn vega(json: &'static str) -> Harness {
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).size(160, 60);
    ui.settle();
    ui
}

fn spot(ui: &mut Harness, role: Role, text: &str) -> Rect {
    let found = ui
        .find_all(&Query::role(role))
        .into_iter()
        .filter(|node| node.name() == Some(text) || node.value() == Some(text))
        .filter_map(|node| node.bounds())
        .collect::<Vec<_>>();
    match found.as_slice() {
        [only] => *only,
        _ => panic!("{} nodes read {text:?}: {}", found.len(), ui.text()),
    }
}

fn label(ui: &mut Harness, text: &str) -> Rect {
    spot(ui, Role::Label, text)
}

fn bar(ui: &mut Harness, caption: &str) -> Rect {
    spot(ui, Role::GenericContainer, caption)
}

fn assert_left_to_right(ui: &mut Harness, names: &[&str]) {
    let xs = names
        .iter()
        .map(|name| label(ui, name).origin.x)
        .collect::<Vec<_>>();
    assert!(
        xs.windows(2).all(|pair| pair[0] < pair[1]),
        "{names:?} sit at x {xs:?}"
    );
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
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).size(160, 60);
    ui.settle();
    let (feb, mar) = (x_of(&mut ui, "February"), x_of(&mut ui, "March"));
    assert!(feb < mar, "February at {feb} is right of March at {mar}");
    assert!(
        ui.query(&Query::text("Jan 1, 2024")).is_none(),
        "raw date label"
    );
}

#[test]
fn vega_area_with_color_stacks() {
    let json = r#"{"data":{"values":[{"x":"p","g":"A","y":6},{"x":"q","g":"A","y":6},{"x":"p","g":"B","y":6},{"x":"q","g":"B","y":6}]},"mark":"area","encoding":{"x":{"field":"x","type":"nominal"},"y":{"field":"y","type":"quantitative"},"color":{"field":"g","type":"nominal"}}}"#;
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).size(160, 60);
    ui.settle();
    assert!(
        ui.query(&Query::text("12")).is_some() && ui.query(&Query::text("20")).is_none(),
        "y axis does not end at the stacked total 12: {}",
        ui.text()
    );
}

#[test]
fn vega_quantitative_x_line_is_spaced_by_value() {
    let json = r#"{"data":{"values":[{"x":1,"y":1},{"x":2,"y":2},{"x":10,"y":3}]},"mark":"line","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#;
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).size(160, 60);
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
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).size(160, 60);
    ui.settle();
    let shown = ui.accessibility_snapshot();
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
    let mut ui = Harness::new(move || markdown(signal(fence(json)))).size(160, 60);
    ui.settle();
    let shown = ui.accessibility_snapshot();
    assert!(!shown.contains("inf"), "axis or caption reads inf: {shown}");
}

#[test]
fn a_nominal_y_draws_horizontal_bars() {
    let mut ui = vega(
        r#"{"data":{"values":[{"a":"C","b":2},{"a":"C","b":7},{"a":"C","b":4},{"a":"D","b":1},{"a":"D","b":2},{"a":"D","b":6},{"a":"E","b":8},{"a":"E","b":4},{"a":"E","b":7}]},"mark":"bar","encoding":{"y":{"field":"a","type":"nominal"},"x":{"aggregate":"average","field":"b","type":"quantitative"}}}"#,
    );
    let (c, d, e) = (
        label(&mut ui, "C"),
        label(&mut ui, "D"),
        label(&mut ui, "E"),
    );
    assert!(
        c.origin.y < d.origin.y && d.origin.y < e.origin.y,
        "C {c:?} D {d:?} E {e:?}"
    );
    let (low, high) = (bar(&mut ui, "4.33"), bar(&mut ui, "6.33"));
    assert!(
        low.width > low.height,
        "the C bar {low:?} is not horizontal"
    );
    assert!(
        (low.origin.x - high.origin.x).abs() < 1.0,
        "bars start apart: {low:?} {high:?}"
    );
    assert!(
        high.width > low.width,
        "6.33 {high:?} is not longer than 4.33 {low:?}"
    );
}

#[test]
fn a_stacked_horizontal_bar_puts_the_first_color_at_zero() {
    let mut ui = vega(
        r#"{"data":{"values":[{"v":"P","s":"Waseca","y":40},{"v":"P","s":"Crookston","y":30},{"v":"Q","s":"Waseca","y":20},{"v":"Q","s":"Crookston","y":10}]},"mark":"bar","encoding":{"x":{"aggregate":"sum","field":"y"},"y":{"field":"v"},"color":{"field":"s"}}}"#,
    );
    let (first, last) = (bar(&mut ui, "Crookston 30"), bar(&mut ui, "Waseca 40"));
    assert!(
        first.origin.x < last.origin.x,
        "Crookston {first:?} is right of Waseca {last:?}"
    );
    assert!(
        (first.origin.y - last.origin.y).abs() < 1.0,
        "P segments are on two rows"
    );
}

#[test]
fn nominal_categories_sort_ascending() {
    let mut ui = vega(
        r#"{"data":{"values":[{"k":"Manchuria","v":3},{"k":"Glabron","v":4},{"k":"Svansota","v":5}]},"mark":"bar","encoding":{"x":{"field":"k","type":"nominal"},"y":{"field":"v","type":"quantitative"}}}"#,
    );
    assert_left_to_right(&mut ui, &["Glabron", "Manchuria", "Svansota"]);
}

#[test]
fn a_sort_array_orders_the_categories() {
    let mut ui = vega(
        r#"{"data":{"values":[{"a":"A","b":28},{"a":"B","b":55},{"a":"C","b":43}]},"mark":"bar","encoding":{"x":{"field":"a","type":"ordinal","sort":["B","A","C"]},"y":{"field":"b","type":"quantitative"}}}"#,
    );
    assert_left_to_right(&mut ui, &["B", "A", "C"]);
}

#[test]
fn a_descending_count_sort_puts_the_biggest_bar_first() {
    let mut ui = vega(
        r#"{"data":{"values":[{"cyl":"a"},{"cyl":"b"},{"cyl":"b"},{"cyl":"b"},{"cyl":"c"},{"cyl":"c"},{"cyl":"d"},{"cyl":"d"},{"cyl":"d"},{"cyl":"d"},{"cyl":"e"},{"cyl":"e"},{"cyl":"e"},{"cyl":"e"},{"cyl":"e"}]},"mark":"bar","encoding":{"x":{"field":"cyl","sort":{"op":"count","order":"descending"}},"y":{"aggregate":"count"}}}"#,
    );
    assert_left_to_right(&mut ui, &["e", "d", "b", "c", "a"]);
}

#[test]
fn a_minus_y_sort_orders_by_value_descending() {
    let mut ui = vega(
        r#"{"data":{"values":[{"k":"p","v":1},{"k":"q","v":9},{"k":"r","v":5}]},"mark":"bar","encoding":{"x":{"field":"k","type":"nominal","sort":"-y"},"y":{"field":"v","type":"quantitative"}}}"#,
    );
    assert_left_to_right(&mut ui, &["q", "r", "p"]);
}

#[test]
fn a_color_legend_sorts_ascending() {
    let mut ui = vega(
        r#"{"data":{"values":[{"x":1,"y":2,"o":"USA"},{"x":2,"y":3,"o":"Japan"},{"x":3,"y":1,"o":"Europe"}]},"mark":"point","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"},"color":{"field":"o","type":"nominal"}}}"#,
    );
    assert_left_to_right(&mut ui, &["Europe", "Japan", "USA"]);
}

#[test]
fn the_first_color_stacks_on_top() {
    let mut ui = vega(
        r#"{"data":{"values":[{"a":"A1","b":28,"c":"x"},{"a":"A1","b":23,"c":"y"},{"a":"A2","b":18,"c":"x"},{"a":"A2","b":21,"c":"y"}]},"mark":"bar","encoding":{"x":{"field":"a","type":"ordinal"},"y":{"field":"b","type":"quantitative","stack":"zero"},"color":{"field":"c","type":"nominal"}}}"#,
    );
    let (top, bottom) = (bar(&mut ui, "x 28"), bar(&mut ui, "y 23"));
    assert!(
        top.origin.y < bottom.origin.y,
        "x {top:?} is below y {bottom:?}"
    );
}
