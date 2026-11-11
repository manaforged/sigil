use sigil::prelude::*;
use sigil::widgets::{ScrollAnchor, virtual_list};
use sigil_kittest::Harness;

fn screen(anchor: ScrollAnchor) -> String {
    let lines: Vec<String> = (0..50).map(|n| format!("line {n}")).collect();
    Harness::new(move || {
        virtual_list(lines, |_, line: &String| text(line.clone()))
            .row_height(rows(1.0))
            .viewport_height(rows(6.0))
            .initial_scroll(anchor)
    })
    .text()
}

#[test]
fn row_viewport_paints_the_first_rows() {
    let screen = screen(ScrollAnchor::Top);
    assert!(screen.contains("line 0"), "screen:\n{screen}");
}

#[test]
fn row_viewport_following_the_end_paints_the_last_row() {
    let screen = screen(ScrollAnchor::Bottom);
    assert!(screen.contains("line 49"), "screen:\n{screen}");
}

#[test]
fn row_viewport_scrolled_to_a_row_paints_that_row() {
    let screen = screen(ScrollAnchor::Index(30));
    assert!(screen.contains("line 30"), "screen:\n{screen}");
}
