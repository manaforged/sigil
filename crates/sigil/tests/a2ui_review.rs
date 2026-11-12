#![allow(clippy::unwrap_used, reason = "test fixtures")]
#![allow(
    clippy::indexing_slicing,
    reason = "a test fails loudly on a missing value"
)]

use std::io::Cursor;
use std::time::{Duration, Instant};

use sigil::prelude::*;
use sigil_kittest::{Harness, Query};

const STACK: usize = 8 << 20;
const BOUND: Duration = Duration::from_secs(5);

fn ui(json: &str) -> Harness {
    let json = json.to_string();
    let mut u = Harness::new(move || a2ui(signal(json.clone()))).size(100, 40);
    u.settle();
    u
}

fn hostile<T: Send + 'static>(run: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap()
}

fn timed(json: String) -> (Duration, String) {
    hostile(move || {
        let start = Instant::now();
        let mut shown = ui(&json);
        let took = start.elapsed();
        (took, shown.accessibility_snapshot())
    })
}

fn surface(components: &str, data: &str) -> String {
    format!(
        r#"[{{"updateComponents":{{"surfaceId":"s","components":{components}}}}},{{"updateDataModel":{{"surfaceId":"s","path":"/","value":{data}}}}}]"#
    )
}

#[test]
fn a_data_path_deeper_than_the_limit_is_rejected() {
    let path = "/a".repeat(100_000);
    let update = format!(
        r#"[{{"updateComponents":{{"surfaceId":"s","components":[{{"id":"root","component":"Text","text":"ok"}}]}}}},{{"updateDataModel":{{"surfaceId":"s","path":"{path}","value":1}}}}]"#
    );
    let shown = hostile(move || ui(&update).text());
    assert!(shown.contains("deeper than"), "{shown}");
    let field = format!(
        r#"{{"updateComponents":{{"surfaceId":"s","components":[{{"id":"root","component":"TextField","label":"Deep","value":{{"path":"{path}"}}}}]}}}}"#
    );
    let typed = hostile(move || {
        let mut form = ui(&field);
        let input = form.get(Query::role(Role::TextInput));
        form.click(&input).type_text("x").settle();
        form.get(Query::role(Role::TextInput))
            .value()
            .map(str::to_owned)
    });
    assert_eq!(typed.as_deref(), Some("x"));
}

#[test]
fn a_component_inside_its_own_ancestors_is_an_error_note() {
    let json = r#"{"updateComponents":{"surfaceId":"s","components":[{"id":"root","component":"Column","children":["root","root"]}]}}"#;
    let (took, shown) = timed(json.to_string());
    eprintln!("cycle first settle: {took:?}");
    assert!(took < BOUND, "cycle settle took {took:?}");
    assert!(shown.contains("contains itself"), "{shown}");
}

#[test]
fn the_widest_allowed_component_tree_settles_in_time() {
    let mut parts: Vec<String> = (0..31)
        .map(|at| {
            let id = if at == 0 {
                "root".to_string()
            } else {
                format!("c{at}")
            };
            let next = format!("c{}", at + 1);
            format!(r#"{{"id":"{id}","component":"Column","children":["{next}","{next}"]}}"#)
        })
        .collect();
    parts.push(r#"{"id":"c31","component":"Text","text":"leaf"}"#.to_string());
    let json = format!(
        r#"{{"updateComponents":{{"surfaceId":"s","components":[{}]}}}}"#,
        parts.join(",")
    );
    let (took, shown) = timed(json);
    eprintln!("branching first settle: {took:?}");
    assert!(took < BOUND, "branching settle took {took:?}");
    assert!(shown.contains("too large"), "{shown}");
}

#[test]
fn nested_templates_cannot_multiply_one_large_text() {
    let big = "word ".repeat((48 << 10) / 5);
    let components = format!(
        r#"[{{"id":"root","component":"Column","children":{{"componentId":"row","path":"/items"}}}},{{"id":"row","component":"Column","children":{{"componentId":"cell","path":"/items"}}}},{{"id":"cell","component":"Text","text":"{big}"}}]"#
    );
    let items: Vec<String> = (0..256).map(|at| at.to_string()).collect();
    let json = surface(
        &components,
        &format!(r#"{{"items":[{}]}}"#, items.join(",")),
    );
    let (took, shown) = timed(json);
    eprintln!("template first settle: {took:?}");
    assert!(took < BOUND, "template settle took {took:?}");
    assert!(shown.contains("too large"), "{shown}");
}

#[test]
fn inputs_bound_to_one_path_show_the_same_value() {
    let field = r#"{"id":"a","component":"TextField","label":"First","value":{"path":"/name"}}"#;
    let other = r#"{"id":"b","component":"TextField","label":"Second","value":{"path":"/name"}}"#;
    let root = r#"{"id":"root","component":"Column","children":["a","b"]}"#;
    let mut shown = ui(&surface(
        &format!("[{root},{field},{other}]"),
        r#"{"name":""}"#,
    ));
    let first = shown.find_all(&Query::role(Role::TextInput)).remove(0);
    shown.click(&first).type_text("Ada").settle();
    let values: Vec<Option<String>> = shown
        .find_all(&Query::role(Role::TextInput))
        .iter()
        .map(|node| node.value().map(str::to_owned))
        .collect();
    assert_eq!(
        values,
        vec![Some("Ada".to_string()), Some("Ada".to_string())]
    );
}

#[test]
fn a_data_path_without_a_leading_slash_is_rejected() {
    let json = r#"[{"updateComponents":{"surfaceId":"s","components":[{"id":"root","component":"Text","text":{"path":"/name"}}]}},{"updateDataModel":{"surfaceId":"s","path":"/name","value":"Ada"}},{"updateDataModel":{"surfaceId":"s","path":"title","value":"x"}}]"#;
    let shown = ui(json).text();
    assert!(shown.contains("not a JSON Pointer"), "{shown}");
}

#[test]
fn array_writes_use_json_pointer_index_rules() {
    let json = r#"[{"updateComponents":{"surfaceId":"s","components":[{"id":"root","component":"Text","text":{"path":"/items/1"}}]}},{"updateDataModel":{"surfaceId":"s","path":"/items","value":["one","two"]}},{"updateDataModel":{"surfaceId":"s","path":"/items/01","value":"nine"}},{"updateDataModel":{"surfaceId":"s","path":"/items/+1","value":"nine"}}]"#;
    let shown = ui(json).text();
    assert!(shown.contains("two"), "{shown}");
    assert!(!shown.contains("nine"), "{shown}");
}

#[test]
fn a_single_choice_picker_leaves_the_model_alone_at_mount() {
    let picker = r#"{"id":"pick","component":"ChoicePicker","label":"Constraint","value":{"path":"/size"},"options":[{"label":"Small","value":"s"},{"label":"Large","value":"l"}]}"#;
    let echo = r#"{"id":"echo","component":"Text","text":{"path":"/size"}}"#;
    let root = r#"{"id":"root","component":"Column","children":["pick","echo"]}"#;
    let shown = ui(&surface(
        &format!("[{root},{picker},{echo}]"),
        r#"{"size":"l"}"#,
    ))
    .text();
    assert!(!shown.contains(r#"["l"]"#), "{shown}");
}

#[test]
fn literal_text_keeps_a_leading_hash() {
    let json = r##"{"updateComponents":{"surfaceId":"s","components":[{"id":"root","component":"Text","text":"#1 priority"}]}}"##;
    let shown = ui(json).text();
    assert!(shown.contains("#1 priority"), "{shown}");
}

#[test]
fn text_shows_a_markdown_link_as_plain_text() {
    let json = r#"{"updateComponents":{"surfaceId":"s","components":[{"id":"root","component":"Text","text":"Read **[the guide](https://example.com/guide)** now."}]}}"#;
    let mut shown = ui(json);
    let screen = shown.text();
    assert!(screen.contains("Read the guide now."), "{screen}");
    assert!(!screen.contains("example.com"), "{screen}");
    assert!(shown.find_all(&Query::role(Role::Link)).is_empty());
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let word = chunk.iter().enumerate().fold(0u32, |word, (at, byte)| {
            word | u32::from(*byte) << (16 - 8 * at)
        });
        for at in 0..4 {
            let sextet = (word >> (18 - 6 * at)) & 63;
            let shown = at <= chunk.len();
            out.push(if shown {
                char::from(ALPHABET[sextet as usize])
            } else {
                '='
            });
        }
    }
    out
}

fn png() -> String {
    let pixels = [10u8, 20, 30, 255].repeat(12);
    let picture = image::RgbaImage::from_raw(4, 3, pixels).unwrap();
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(picture)
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    base64(&bytes)
}

fn images(mut shown: Harness) -> usize {
    shown.find_all(&Query::role(Role::Image)).len()
}

#[test]
fn a2ui_and_markdown_accept_the_same_data_uris() {
    let payload = png();
    for meta in [
        "image/png;base64",
        "image/png;base64x",
        "image/svg+xml;base64",
    ] {
        let uri = format!("data:{meta},{payload}");
        let json = format!(
            r#"{{"updateComponents":{{"surfaceId":"s","components":[{{"id":"root","component":"Image","url":"{uri}","description":"pic"}}]}}}}"#
        );
        let source = format!("![pic]({uri})");
        let mut prose = Harness::new(move || markdown(signal(source.clone()))).size(100, 40);
        prose.settle();
        let (surface, prose) = (images(ui(&json)), images(prose));
        assert_eq!(surface, prose, "{meta}");
        assert_eq!(surface, usize::from(meta == "image/png;base64"), "{meta}");
    }
}

#[test]
fn a_version_1_binding_and_a_version_0_9_binding_both_show_their_data() {
    let json = surface(
        r#"[{"id":"root","component":"Column","children":["new","old"]},{"id":"new","component":"Text","text":{"@path":"/title"}},{"id":"old","component":"Text","text":{"path":"/owner"}}]"#,
        r#"{"title":"Fix cart rounding","owner":"sam"}"#,
    );
    let shown = ui(&json).text();
    assert!(shown.contains("Fix cart rounding"), "{shown}");
    assert!(shown.contains("sam"), "{shown}");
}
