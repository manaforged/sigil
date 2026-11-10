#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic, reason = "a test fails loudly on a missing value")]

use sigil::geometry::Rect;
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil_kittest::{Harness, Query};

const ROW: f32 = 20.0;

fn vega(json: String) -> Harness {
    let mut ui =
        Harness::new(move || markdown(signal(format!("```vega-lite\n{json}\n```")))).size(160, 60);
    ui.settle();
    ui
}

fn labels(ui: &mut Harness) -> Vec<(String, Rect)> {
    ui.find_all(&Query::role(Role::Label))
        .into_iter()
        .filter_map(|node| Some((node.value()?.to_owned(), node.bounds()?)))
        .collect()
}

fn has(ui: &mut Harness, text: &str) -> bool {
    labels(ui).iter().any(|(value, _)| value == text)
}

fn label(ui: &mut Harness, text: &str) -> Rect {
    let found = labels(ui)
        .into_iter()
        .filter(|(value, _)| value == text)
        .collect::<Vec<_>>();
    match found.as_slice() {
        [(_, rect)] => *rect,
        _ => panic!("{} labels read {text:?}: {}", found.len(), ui.text()),
    }
}

fn bars(ui: &mut Harness) -> Vec<Rect> {
    ui.find_all(&Query::role(Role::GenericContainer))
        .into_iter()
        .filter(|node| node.name().is_some())
        .filter_map(|node| node.bounds())
        .collect()
}

#[test]
fn a_y_scale_domain_sets_the_y_ticks() {
    let mut ui = vega(r#"{"data":{"values":[{"x":1,"v":1},{"x":2,"v":3},{"x":3,"v":2}]},"mark":"line","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"v","type":"quantitative","scale":{"domain":[0,5]}}}}"#.to_owned());
    for tick in ["0.0", "2.5", "4.5", "5.0"] {
        assert!(has(&mut ui, tick), "no y tick {tick}: {}", ui.text());
    }
}

#[test]
fn an_x_scale_domain_sets_the_x_ticks() {
    let mut ui = vega(r#"{"data":{"values":[{"w":250,"p":1},{"w":300,"p":2},{"w":350,"p":3},{"w":400,"p":2},{"w":500,"p":1}]},"mark":"line","encoding":{"x":{"field":"w","type":"quantitative","scale":{"domain":[300,450]}},"y":{"field":"p","type":"quantitative"}}}"#.to_owned());
    assert!(
        has(&mut ui, "300") && has(&mut ui, "440"),
        "x ticks: {}",
        ui.text()
    );
    assert!(
        !has(&mut ui, "250") && !has(&mut ui, "500"),
        "x ticks leave the domain: {}",
        ui.text()
    );
}

#[test]
fn a_temporal_x_reads_as_time_ticks() {
    let mut ui = vega(r#"{"width":800,"height":300,"data":{"values":[{"ts":1500400000000,"v":1},{"ts":1500500000000,"v":2},{"ts":1500600000000,"v":3},{"ts":1500700000000,"v":2}]},"mark":"line","encoding":{"x":{"field":"ts","type":"temporal","scale":{"domain":[1500000000000,1500900000000]}},"y":{"field":"v","type":"quantitative"}}}"#.to_owned());
    assert!(
        !has(&mut ui, "1500400000000"),
        "raw epoch label: {}",
        ui.text()
    );
    let (saturday, monday) = (label(&mut ui, "Sat 15"), label(&mut ui, "Mon 17"));
    assert!(has(&mut ui, "Jul 16"), "no Sunday tick: {}", ui.text());
    assert!(
        saturday.origin.x < monday.origin.x,
        "Sat 15 {saturday:?} right of Mon 17 {monday:?}"
    );
}

#[test]
fn the_y_axis_ends_near_the_data_maximum() {
    let mut ui = vega(r#"{"data":{"values":[{"c":"A","g":"x","v":0.1},{"c":"A","g":"y","v":0.6},{"c":"B","g":"x","v":0.7},{"c":"B","g":"y","v":1.1}]},"mark":"bar","encoding":{"x":{"field":"c"},"y":{"field":"v","type":"quantitative"},"xOffset":{"field":"g"},"color":{"field":"g"}}}"#.to_owned());
    assert!(has(&mut ui, "1.1"), "no 1.1 tick: {}", ui.text());
    assert!(
        !has(&mut ui, "2") && !has(&mut ui, "1.5"),
        "the axis overshoots: {}",
        ui.text()
    );
    let mut bars = vega(r#"{"height":200,"data":{"values":[{"k":"Europe","v":73},{"k":"USA","v":254}]},"mark":"bar","encoding":{"x":{"field":"k"},"y":{"field":"v","type":"quantitative"}}}"#.to_owned());
    assert!(
        has(&mut bars, "250") && has(&mut bars, "50"),
        "ticks: {}",
        bars.text()
    );
    assert!(
        !has(&mut bars, "500"),
        "the axis doubles the maximum: {}",
        bars.text()
    );
}

#[test]
fn a_negative_domain_ticks_through_zero() {
    let mut ui = vega(r#"{"data":{"values":[{"a":"A","b":-28},{"a":"B","b":55},{"a":"C","b":-33},{"a":"D","b":91}]},"mark":"bar","encoding":{"x":{"field":"a","type":"nominal"},"y":{"field":"b","type":"quantitative"}}}"#.to_owned());
    for tick in ["-40", "-20", "0", "20", "100"] {
        assert!(has(&mut ui, tick), "no tick {tick}: {}", ui.text());
    }
    assert!(
        !has(&mut ui, "-50") && !has(&mut ui, "25"),
        "off-step ticks: {}",
        ui.text()
    );
}

#[test]
fn both_axes_show_their_titles() {
    let mut ui = vega(r#"{"data":{"values":[{"variety":"P","yield":3},{"variety":"Q","yield":4}]},"mark":"bar","encoding":{"x":{"field":"variety"},"y":{"aggregate":"sum","field":"yield"}}}"#.to_owned());
    assert!(
        has(&mut ui, "variety") && has(&mut ui, "Sum of yield"),
        "titles: {}",
        ui.text()
    );
    let mut explicit = vega(r#"{"data":{"values":[{"a":"A","b":28}]},"mark":"bar","encoding":{"x":{"field":"a","type":"nominal","title":"\"Quoted\" Title"},"y":{"field":"b","type":"quantitative","axis":{"title":null}}}}"#.to_owned());
    assert!(
        has(&mut explicit, "\"Quoted\" Title"),
        "explicit title: {}",
        explicit.text()
    );
    assert!(
        !has(&mut explicit, "b"),
        "a null axis title still shows: {}",
        explicit.text()
    );
    let mut counted = vega(r#"{"data":{"values":[{"o":"USA"},{"o":"Japan"}]},"mark":"bar","encoding":{"x":{"field":"o"},"y":{"aggregate":"count"}}}"#.to_owned());
    assert!(
        has(&mut counted, "Count of Records"),
        "count title: {}",
        counted.text()
    );
}

#[test]
fn a_chart_without_height_has_a_300_px_plot() {
    let mut ui = vega(r#"{"data":{"values":[{"k":"a","v":100},{"k":"b","v":40}]},"mark":"bar","encoding":{"x":{"field":"k"},"y":{"field":"v","type":"quantitative"}}}"#.to_owned());
    let (zero, top) = (label(&mut ui, "0"), label(&mut ui, "100"));
    let span = zero.origin.y - top.origin.y;
    assert!((span - 300.0).abs() <= ROW, "0 to 100 spans {span} px");
}

#[test]
fn bars_on_a_temporal_x_have_width() {
    let rows = (1970..1982)
        .map(|year| format!(r#"{{"Year":"{year}-01-01","m":{}}}"#, year - 1950))
        .collect::<Vec<_>>()
        .join(",");
    let mut ui = vega(format!(
        r#"{{"width":150,"height":150,"data":{{"values":[{rows}]}},"mark":"bar","encoding":{{"x":{{"field":"Year","type":"temporal"}},"y":{{"field":"m","type":"quantitative","aggregate":"average"}}}}}}"#
    ));
    let drawn = bars(&mut ui);
    assert_eq!(drawn.len(), 12, "bars: {}", ui.text());
    assert!(
        drawn
            .iter()
            .all(|rect| rect.width >= 1.0 && rect.height >= 1.0),
        "{drawn:?}"
    );
    assert!(
        has(&mut ui, "1970") && !has(&mut ui, "1970-01-01"),
        "labels: {}",
        ui.text()
    );
}

#[test]
fn three_hundred_categories_in_120_px_still_draw() {
    let rows = (0..300)
        .map(|at| format!(r#"{{"Name":"car {at}"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let mut ui = vega(format!(
        r#"{{"width":120,"height":120,"data":{{"values":[{rows}]}},"mark":"bar","encoding":{{"x":{{"field":"Name"}},"y":{{"aggregate":"count"}}}}}}"#
    ));
    let drawn = bars(&mut ui);
    assert_eq!(drawn.len(), 300, "bars: {}", drawn.len());
    assert!(
        drawn.iter().all(|rect| rect.width > 0.99),
        "a bar is under 1 px wide"
    );
}
