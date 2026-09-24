use crate::kernel::Cx;
use crate::reactive::{Runtime, signal};

use super::html;
use super::parse::parse_html;
use crate::widgets::markdown::parse::Block;

#[test]
fn html_builds() {
    Runtime::scope("html-builds", || {
        let source = signal(String::from("<h1>Hi</h1><p>there</p>"));
        let mut cx = Cx::new_root();
        let id = cx.add(html(source));
        cx.flush_reactive_children();
        let node = cx.get(id).expect("html root");
        assert!(!node.children().is_empty(), "html must build blocks");
    });
}

#[test]
fn html_maps_subset_to_markdown_blocks() {
    let blocks = parse_html(
        "<h1>Title</h1><p>Hello <strong>world</strong></p><ul><li>a</li><li>b</li></ul>",
    );
    assert!(matches!(
        blocks.first(),
        Some(Block::Heading { level: 1, .. })
    ));
    assert!(matches!(blocks.get(1), Some(Block::Paragraph(body)) if body.contains("**world**")));
    assert!(matches!(blocks.get(2), Some(Block::List(list)) if list.items.len() == 2));
}

#[test]
fn html_fixture_matches_markdown_blocks() {
    let html = include_str!("html/notes.html");
    let md = include_str!("html/notes.md");
    assert_eq!(
        parse_html(html),
        crate::widgets::markdown::parse::parse_blocks(md),
        "html() must emit the same blocks as the matching markdown"
    );
}

#[test]
fn html_drops_script() {
    let blocks = parse_html("<p>ok</p><script>alert(1)</script><p>still</p>");
    let text: Vec<_> = blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(body) => Some(body.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, ["ok", "still"]);
}
