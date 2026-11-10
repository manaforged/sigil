use super::super::containers::{List, ListItem, TaskMark};
use super::super::table_data::{Align, Table};
use super::{Block, Inline, parse_blocks, parse_inlines};

fn text(body: &str) -> Vec<Inline> {
    vec![Inline::Text(body.into())]
}

fn hello() -> Vec<Inline> {
    vec![
        Inline::Text("Hello ".into()),
        Inline::Strong(text("world")),
        Inline::Text(".".into()),
    ]
}

#[test]
fn parse_headings_list_fence_and_paragraph() {
    let blocks =
        parse_blocks("# Title\n\nHello **world**.\n\n- one\n- two\n\n```\nfn x() {}\n```\n");
    assert_eq!(
        blocks,
        vec![
            Block::Heading {
                level: 1,
                body: text("Title"),
            },
            Block::Paragraph(hello()),
            Block::List(List {
                start: None,
                items: vec![
                    ListItem::plain(vec![Block::Paragraph(text("one"))]),
                    ListItem::plain(vec![Block::Paragraph(text("two"))]),
                ],
            }),
            Block::Code {
                lang: String::new(),
                body: "fn x() {}".into(),
            },
        ]
    );
    assert_eq!(parse_inlines("Hello **world**."), hello());
    assert_eq!(
        parse_inlines("see [docs](https://example.com)"),
        vec![
            Inline::Text("see ".into()),
            Inline::Link {
                href: "https://example.com".into(),
                body: text("docs"),
                label_is_href: false,
            },
        ]
    );
    assert_eq!(
        parse_blocks("| a | b |\n| --- | --- |\n| 1 | 2 |\n"),
        vec![Block::Table(Table {
            align: vec![Align::Start; 2],
            rows: vec![vec![text("a"), text("b")], vec![text("1"), text("2")]],
        })]
    );
}

fn tasks(source: &str) -> Vec<(Option<TaskMark>, Vec<Block>)> {
    match parse_blocks(source).into_iter().next() {
        Some(Block::List(list)) => list
            .items
            .into_iter()
            .map(|ListItem { task, blocks }| (task, blocks))
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn task_items_carry_checked_state_and_drop_the_marker() {
    assert_eq!(
        tasks("- [ ] todo\n- [x] done\n- [X] loud\n- plain [x]\n"),
        vec![
            (Some(TaskMark::Open), vec![Block::Paragraph(text("todo"))]),
            (Some(TaskMark::Done), vec![Block::Paragraph(text("done"))]),
            (Some(TaskMark::Done), vec![Block::Paragraph(text("loud"))]),
            (None, vec![Block::Paragraph(text("plain [x]"))]),
        ]
    );
}

#[test]
fn a_star_between_spaces_is_text_not_emphasis() {
    assert_eq!(parse_inlines("a * b"), text("a * b"));
    assert_eq!(parse_inlines("a ** b"), text("a ** b"));
    assert_eq!(parse_inlines("2 * 3 * 4"), text("2 * 3 * 4"));
    assert_eq!(
        parse_inlines("*x* and **y**"),
        vec![
            Inline::Em(text("x")),
            Inline::Text(" and ".into()),
            Inline::Strong(text("y")),
        ]
    );
}

#[test]
fn a_mermaid_fence_holding_an_xychart_parses_as_a_chart() {
    let blocks = parse_blocks(
        "```mermaid\nxychart-beta\n    title \"Rust lines by folder\"\n    x-axis [\"tui\", \"provider\", \"tools\"]\n    y-axis \"Lines\" 0 --> 40000\n    bar [36704, 11307, 8503]\n```\n",
    );
    assert!(
        matches!(
            blocks.as_slice(),
            [Block::Chart(data)]
                if data.labels == ["tui", "provider", "tools"]
                    && data.values == [36704.0, 11307.0, 8503.0]
                    && data.title.as_deref() == Some("Rust lines by folder")
        ),
        "not the chart: {blocks:?}"
    );
}
