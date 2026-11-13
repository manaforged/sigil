#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test fails loudly on a missing value"
)]

use std::collections::HashMap;
use std::net::TcpListener;
use std::time::{Duration, Instant};

use accesskit::{Node, NodeId, Role as AkRole};
use sigil::prelude::*;
use sigil::scene::SceneCommand;
use sigil_kittest::{Harness, Query};

const RENDERING: &str = "Rendering chart";
const UNTITLED: &str = "Chart";
const WAIT: Duration = Duration::from_secs(10);
const DEADLINE_BOUND: Duration = Duration::from_millis(3500);
const MEMORY_BOUND: Duration = Duration::from_millis(1500);
const THEME_SWITCH_BOUND: Duration = Duration::from_millis(500);

const CPU_BOMB: &str = r#"{"data":{"sequence":{"start":0,"stop":40000,"as":"x"}},"transform":[{"window":[{"op":"median","field":"x","as":"m"}],"frame":[-20000,20000]}],"mark":"point","encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"m","type":"quantitative"}}}"#;
const RANDOM: &str = r#"{"data":{"sequence":{"start":0,"stop":6,"as":"i"}},"transform":[{"calculate":"floor(random()*1000000)","as":"r"}],"mark":"text","encoding":{"y":{"field":"i","type":"ordinal"},"text":{"field":"r","type":"quantitative","format":"d"}}}"#;
const TITLED: &str = r#"{"title":"Yield by site","description":"Barley yield per site.","data":{"values":[{"site":"Morris","yield":28},{"site":"Crookston","yield":55}]},"mark":"bar","encoding":{"x":{"field":"site","type":"nominal","title":"Site"},"y":{"field":"yield","type":"quantitative","title":"Yield (bushels)"}}}"#;

#[derive(Debug, PartialEq, Eq)]
enum Shown {
    Figure { name: String, lines: Vec<String> },
    NativeChart,
    Code,
    Nothing,
}

fn memory_bomb() -> String {
    let double = r#"{"calculate":"datum.s+datum.s","as":"s"}"#;
    format!(
        r#"{{"data":{{"values":[{{"s":"xxxxxxxx"}}]}},"transform":[{},{{"calculate":"indexof(datum.s,'y')","as":"i"}}],"mark":"point"}}"#,
        [double; 26].join(",")
    )
}

fn bar(first: &str, second: &str) -> String {
    format!(
        r#"{{"data":{{"values":[{{"a":"{first}","b":28}},{{"a":"{second}","b":55}}]}},"mark":"bar","encoding":{{"x":{{"field":"a","type":"nominal","title":"{first}{second}"}},"y":{{"field":"b","type":"quantitative"}}}}}}"#
    )
}

fn fence(json: &str) -> String {
    format!("```vega-lite\n{json}\n```")
}

fn drawn(json: &str) -> (Harness, Duration) {
    let source = fence(json);
    let start = Instant::now();
    let mut ui = Harness::new(move || markdown(signal(source.clone()))).pixels(1600, 1200);
    ui.settle();
    while ui.query(&Query::text(RENDERING)).is_some() {
        assert!(start.elapsed() < WAIT, "still rendering after {WAIT:?}");
        std::thread::sleep(Duration::from_millis(5));
        ui.settle();
    }
    (ui, start.elapsed())
}

fn mount(json: &str) -> (Shown, Duration) {
    let (mut ui, took) = drawn(json);
    (shown(&mut ui), took)
}

fn holds_image(nodes: &HashMap<NodeId, &Node>, node: &Node) -> bool {
    node.children().iter().any(|child| {
        nodes
            .get(child)
            .is_some_and(|child| child.role() == AkRole::Image || holds_image(nodes, child))
    })
}

fn shown(ui: &mut Harness) -> Shown {
    let tree = ui.accessibility_tree();
    let nodes: HashMap<NodeId, &Node> = tree.nodes.iter().map(|(id, node)| (*id, node)).collect();
    let roles = |role| nodes.values().filter(move |node| node.role() == role);
    if let Some(node) = roles(AkRole::Figure).find(|node| holds_image(&nodes, node)) {
        return Shown::Figure {
            name: node.label().unwrap_or_default().to_string(),
            lines: node
                .description()
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect(),
        };
    }
    if roles(AkRole::Figure).next().is_some() {
        return Shown::NativeChart;
    }
    if roles(AkRole::Code).next().is_some() {
        return Shown::Code;
    }
    Shown::Nothing
}

fn gallery(file: &str) -> String {
    let path = format!(
        "{}/tests/data/vega/{file}.vl.json",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).expect("gallery spec")
}

fn description(json: &str) -> String {
    let spec: serde_json::Value = serde_json::from_str(json).expect("json");
    spec["description"]
        .as_str()
        .expect("description")
        .to_string()
}

fn figure_problem(json: &str, name: &str, about: Option<&str>, titles: &[&str]) -> Option<String> {
    let (shown, _) = mount(json);
    let Shown::Figure { name: got, lines } = &shown else {
        return Some(format!("did not draw a Vega figure: {shown:?}"));
    };
    if got != name {
        return Some(format!("drew a figure named {got:?}, not {name:?}"));
    }
    let drawn = lines
        .iter()
        .map(String::as_str)
        .skip(usize::from(about.is_some()));
    if about.is_some_and(|about| lines.first().map(String::as_str) != Some(about)) {
        return Some(format!("describes itself as {lines:?}"));
    }
    let drawn: Vec<&str> = drawn.collect();
    let missing: Vec<&&str> = titles
        .iter()
        .filter(|title| !drawn.contains(title))
        .collect();
    (!missing.is_empty()).then(|| format!("hides axis titles {missing:?}: {lines:?}"))
}

fn assert_figure(json: &str, name: &str, about: Option<&str>, titles: &[&str]) {
    if let Some(problem) = figure_problem(json, name, about, titles) {
        panic!("{problem}");
    }
}

#[test]
fn gallery_layer_concat_facet_boxplot_time_unit_and_filter_specs_draw_a_figure() {
    let _serial = crate::serial::vega_alone();
    let cases: [(&str, &[&str]); 6] = [
        ("layer_bar_labels", &["a", "b"]),
        ("concat_bar_scales_discretize", &[]),
        ("facet_grid_bar", &["Factor A", "Factor B"]),
        ("boxplot_2D_vertical_single_per_group", &["a", "b"]),
        ("time_output_utc_timeunit", &["price"]),
        ("bar_filter_calc", &["a", "b2"]),
    ];
    let problems: Vec<String> = cases
        .iter()
        .filter_map(|(file, titles)| {
            let json = gallery(file);
            let about = description(&json);
            figure_problem(&json, UNTITLED, Some(&about), titles).map(|p| format!("{file}: {p}"))
        })
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn a_drawn_chart_is_a_figure_named_by_its_title_and_described_by_its_axes() {
    let _serial = crate::serial::vega_alone();
    assert_figure(
        TITLED,
        "Yield by site",
        Some("Barley yield per site."),
        &["Site", "Yield (bushels)"],
    );
    assert_figure(&bar("G", "H"), UNTITLED, None, &["GH", "b"]);
}

#[test]
fn a_cpu_bomb_stops_within_the_deadline_and_the_next_chart_still_draws() {
    let _serial = crate::serial::vega_alone();
    let (shown, took) = mount(CPU_BOMB);
    assert!(
        !matches!(shown, Shown::Figure { .. }),
        "the CPU bomb drew a figure: {shown:?}"
    );
    assert!(took < DEADLINE_BOUND, "the CPU bomb ran for {took:?}");
    assert_figure(&bar("C", "D"), UNTITLED, None, &["CD"]);
}

#[test]
fn a_memory_bomb_falls_back_and_the_next_chart_still_draws() {
    let _serial = crate::serial::vega_alone();
    let (shown, took) = mount(&memory_bomb());
    assert!(
        !matches!(shown, Shown::Figure { .. }),
        "the memory bomb drew a figure: {shown:?}"
    );
    assert!(took < MEMORY_BOUND, "the memory bomb ran for {took:?}");
    assert_figure(&bar("E", "F"), UNTITLED, None, &["EF"]);
}

#[test]
fn a_data_url_falls_back_to_code_without_touching_the_network() {
    let _serial = crate::serial::vega_alone();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.set_nonblocking(true).expect("nonblocking");
    let port = listener.local_addr().expect("addr").port();
    let json = format!(
        r#"{{"data":{{"url":"http://127.0.0.1:{port}/cars.json"}},"mark":"point","encoding":{{"x":{{"field":"a","type":"quantitative"}}}}}}"#
    );
    let (shown, _) = mount(&json);
    assert_eq!(shown, Shown::Code);
    assert!(
        listener.accept().is_err(),
        "the chart opened a connection to its data URL"
    );
}

#[test]
#[ignore = "needs a wgpu adapter"]
fn two_renders_of_a_random_transform_paint_the_same_pixels() {
    let _serial = crate::serial::vega_alone();
    let (mut first, _) = drawn(RANDOM);
    let (mut second, _) = drawn(&format!("{RANDOM} "));
    assert!(
        matches!(shown(&mut first), Shown::Figure { .. }),
        "the random spec fell back"
    );
    let (a, b) = (first.image(), second.image());
    let (a, b) = (
        a.expect("render").expect("gpu"),
        b.expect("render").expect("gpu"),
    );
    assert!(a == b, "two renders of the same random spec differ");
}

fn chart_paths(ui: &mut Harness) -> Vec<String> {
    ui.host()
        .emit((1600.0, 1200.0))
        .commands()
        .iter()
        .filter_map(|command| match command {
            SceneCommand::Path(draw) => Some(format!("{draw:?}")),
            _ => None,
        })
        .collect()
}

fn wait_for_paths(ui: &mut Harness, unlike: &[String]) -> Vec<String> {
    let start = Instant::now();
    loop {
        let paths = chart_paths(ui);
        if !paths.is_empty() && paths != unlike {
            return paths;
        }
        assert!(
            start.elapsed() < WAIT,
            "no new chart drawing after {WAIT:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
        ui.settle();
    }
}

#[test]
fn a_custom_theme_renders_off_the_ui_thread_and_keeps_the_last_image() {
    let _serial = crate::serial::vega_alone();
    let chart = fence(&bar("Custom", "Theme"));
    let held = std::rc::Rc::new(std::cell::Cell::new(None));
    let slot = std::rc::Rc::clone(&held);
    let first = chart.clone();
    let mut ui = Harness::new(move || {
        let source = signal(first.clone());
        slot.set(Some(source));
        markdown(source)
    })
    .pixels(1600, 1200);
    ui.settle();
    let light = wait_for_paths(&mut ui, &[]);
    let busy = fence(&CPU_BOMB.replace("40000", "40001"));
    held.get()
        .expect("source")
        .set(format!("{chart}\n\n{busy}"));
    ui.settle();
    assert_eq!(
        chart_paths(&mut ui),
        light,
        "adding a chart redrew the first one"
    );
    assert!(
        ui.query(&Query::text(RENDERING)).is_some(),
        "the busy chart finished before the theme switch"
    );
    Theme::console_dark().apply();
    let start = Instant::now();
    ui.settle();
    let shown = chart_paths(&mut ui);
    let took = start.elapsed();
    assert!(
        took < THEME_SWITCH_BOUND,
        "the theme switch blocked the frame for {took:?}"
    );
    assert_eq!(
        shown, light,
        "the chart drawing went away while its theme rendered"
    );
    let themed = wait_for_paths(&mut ui, &light);
    assert_eq!(
        themed.len(),
        light.len(),
        "the themed chart replaced the wrong drawing"
    );
}
