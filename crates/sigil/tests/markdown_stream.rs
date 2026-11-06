#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil_kittest::Harness;

fn streamed(chunks: &[&str]) -> String {
    Owner::scope("markdown-stream", || {
        let source = signal(String::new());
        let mut ui = Harness::new(move || markdown(source)).size(80, 30);
        ui.settle();
        let mut body = String::new();
        for chunk in chunks {
            body.push_str(chunk);
            source.set(body.clone());
            ui.settle();
        }
        format!("{}\n{}", ui.text(), ui.accessibility_snapshot())
    })
}

fn full(source: &str) -> String {
    streamed(&[source])
}

fn same_as_full(chunks: &[&str]) -> String {
    let whole = full(&chunks.concat());
    let parts = streamed(chunks);
    assert_eq!(parts, whole, "streamed render differs from a full parse");
    whole
}

#[test]
fn a_footnote_defined_after_its_reference_resolves_while_streaming() {
    let shown = same_as_full(&["See this[^1].\n\n", "Next para.\n\n", "[^1]: The note.\n"]);
    assert!(!shown.contains("[^1]"), "literal footnote reference:\n{shown}");
}

#[test]
fn a_link_defined_after_its_reference_resolves_while_streaming() {
    let shown = same_as_full(&[
        "Read [the guide][g].\n\n",
        "Next.\n\n",
        "[g]: https://example.com/guide\n",
    ]);
    assert!(!shown.contains("[g]"), "literal link reference:\n{shown}");
}

#[test]
fn a_list_item_after_a_blank_line_joins_the_list_while_streaming() {
    let shown = same_as_full(&["1. alpha\n\n2", ". beta\n"]);
    assert!(shown.contains("2. beta"), "second item missing:\n{shown}");
}

#[test]
fn tail_repair_leaves_finished_lines_alone() {
    let shown = full("- a **b\n- c");
    assert!(!shown.contains("c**"), "repair closed the wrong line:\n{shown}");
    assert!(shown.contains("a **b"), "open marker must stay literal:\n{shown}");
}

#[test]
fn deeply_nested_details_render_without_overflow() {
    let shown = full(&"<details>".repeat(20_000));
    assert!(shown.contains("Details"), "no details shown:\n{shown}");
}

#[test]
fn text_past_the_nesting_limit_is_kept() {
    let shown = full(&("- ".repeat(20) + "a"));
    let screen = shown.split('\n').take(30).collect::<Vec<_>>().join("\n");
    assert!(screen.contains('a'), "flattened text lost:\n{shown}");
}
