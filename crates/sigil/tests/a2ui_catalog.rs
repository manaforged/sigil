#![allow(clippy::unwrap_used, reason = "test fixtures")]

use std::cell::RefCell;
use std::rc::Rc;

use sigil::prelude::*;
use sigil_kittest::Harness;

fn surface(components: &str, data: &str) -> String {
    format!(
        r#"[{{"version":"1.0","updateComponents":{{"surfaceId":"s","components":{components}}}}},{{"version":"1.0","updateDataModel":{{"surfaceId":"s","path":"/","value":{data}}}}}]"#
    )
}

const PIECES: &str = r##"[{"id":"root","component":"Column","children":["stat","table","notes","chart","row"]},
{"id":"stat","component":"Stat","label":"p50","value":{"@path":"/p50"},"tone":"success"},
{"id":"table","component":"Table","columns":["host","state"],"rows":{"@path":"/hosts"}},
{"id":"notes","component":"Markdown","text":"**all green**"},
{"id":"chart","component":"Chart","variant":"line","labels":["Mon","Tue"],"series":[{"name":"p50","values":[41.0,38.5]}]},
{"id":"row","component":"Item","title":{"@path":"/issue"},"status":"#212","tone":"warning",
 "action":{"event":{"name":"start","context":{"prompt":{"@path":"/prompt"}}}}}]"##;

const DATA: &str = r#"{"p50":"38 ms","hosts":[["web-1","up"],["web-2","down"]],"issue":"Checkout total off by a cent","prompt":"Fix issue #212"}"#;

#[test]
fn catalog_components_show_their_data() {
    let json = surface(PIECES, DATA);
    let mut shown = Harness::new(move || a2ui(signal(json.clone()))).size(100, 60);
    shown.settle();
    let text = shown.text();
    for expected in [
        "p50",
        "38 ms",
        "host",
        "web-2",
        "down",
        "all green",
        "Checkout total off by a cent",
        "#212",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in {text}");
    }
    assert!(!text.contains("Unsupported"), "{text}");
}

#[test]
fn an_item_with_an_event_sends_its_name_and_bound_context() {
    let json = surface(PIECES, DATA);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&seen);
    let mut shown = Harness::new(move || {
        let sink = Rc::clone(&sink);
        a2ui(signal(json.clone())).activate(move |name, context| {
            sink.borrow_mut()
                .push((name.to_string(), context.to_string()));
        })
    })
    .size(100, 60);
    shown.settle();
    let row = shown.get("Checkout total off by a cent");
    shown.click(&row).settle();
    assert_eq!(
        *seen.borrow(),
        vec![(
            "start".to_string(),
            r#"{"prompt":"Fix issue #212"}"#.to_string()
        )]
    );
}

#[test]
fn every_component_in_the_published_catalog_renders() {
    let catalog: serde_json::Value = serde_json::from_str(sigil::widgets::A2UI_CATALOG).unwrap();
    let names: Vec<String> = catalog
        .get("components")
        .and_then(serde_json::Value::as_object)
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert!(!names.is_empty());
    for name in names {
        let component = format!(
            r#"[{{"id":"root","component":"{name}","title":"x","label":"x","value":"x","columns":[],"rows":[],"text":"x","source":"graph TD; A-->B","patch":"","series":[]}}]"#
        );
        let json = surface(&component, "{}");
        let mut shown = Harness::new(move || a2ui(signal(json.clone()))).size(100, 40);
        shown.settle();
        assert!(
            !shown.text().contains("Unsupported"),
            "{name} is in the catalog but does not render"
        );
    }
}
