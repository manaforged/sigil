use crate::kernel::Cx;
use crate::reactive::{Runtime, signal};

use super::{markdown, markdown_edit};

#[test]
fn markdown_builds() {
    Runtime::scope("markdown-builds", || {
        let source = signal(String::from("# Hi\n\nthere"));
        let mut cx = Cx::new_root();
        let id = cx.add(markdown(source));
        cx.flush_reactive_children();
        let node = cx.get(id).expect("markdown root");
        assert!(!node.children().is_empty(), "markdown must build blocks");
    });
}

#[test]
fn oversized_markdown_source_is_rejected() {
    let big = "a".repeat((1 << 20) + 1);
    let blocks = crate::widgets::markdown::parse::parse_blocks(&big);
    assert!(
        matches!(
            blocks.as_slice(),
            [crate::widgets::markdown::parse::Block::Paragraph(text)]
                if text.contains("1 MiB")
        ),
        "oversized input must return the limit message",
    );
}

#[test]
fn markdown_edit_builds() {
    Runtime::scope("markdown-edit-builds", || {
        let source = signal(String::from("# Hi"));
        let mut cx = Cx::new_root();
        let id = cx.add(markdown_edit(source));
        let node = cx.get(id).expect("markdown edit root");
        assert!(
            !node.children().is_empty(),
            "the editor must build its panes"
        );
    });
}
