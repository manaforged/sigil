use super::super::containers::List;
use super::{Block, Span, parse_blocks, parse_inlines};

#[test]
fn parse_headings_list_fence_and_paragraph() {
    let blocks =
        parse_blocks("# Title\n\nHello **world**.\n\n- one\n- two\n\n```\nfn x() {}\n```\n");
    assert_eq!(
        blocks,
        vec![
            Block::Heading {
                level: 1,
                text: "Title".into(),
            },
            Block::Paragraph("Hello **world**.".into()),
            Block::List(List {
                start: None,
                items: vec![
                    vec![Block::Paragraph("one".into())],
                    vec![Block::Paragraph("two".into())],
                ],
            }),
            Block::Code {
                lang: String::new(),
                body: "fn x() {}".into(),
            },
        ]
    );
    assert_eq!(
        parse_inlines("Hello **world**."),
        vec![
            Span::Text("Hello ".into()),
            Span::Bold("world".into()),
            Span::Text(".".into()),
        ]
    );
    assert_eq!(
        parse_inlines("see [docs](https://example.com)"),
        vec![
            Span::Text("see ".into()),
            Span::Link {
                text: "docs".into(),
                href: "https://example.com".into(),
            },
        ]
    );
    assert_eq!(
        parse_blocks("| a | b |\n| --- | --- |\n| 1 | 2 |\n"),
        vec![Block::Table(vec![
            vec!["a".into(), "b".into()],
            vec!["1".into(), "2".into()],
        ])]
    );
}
