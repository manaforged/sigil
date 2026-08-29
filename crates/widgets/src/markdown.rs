pub(crate) mod parse;

use sigil_kernel::{NodeBuilder, Role, container, text};
use sigil_layout::FlexWrap;
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::NodeStyleExt;

use parse::{Block, Span, parse_blocks, parse_inlines};

#[must_use]
pub fn markdown(source: Signal<String>) -> NodeBuilder {
    let blocks = derived_eq(move || parse_blocks(&source.get()));
    document(blocks)
}

pub(crate) fn document(blocks: Signal<Vec<Block>>) -> NodeBuilder {
    container()
        .role(Role::Document)
        .layout(LayoutStyle::flex_column().fill_width().gap(8.0))
        .children_reactive(move || blocks.get().into_iter().map(block_node).collect::<Vec<_>>())
}

#[must_use]
pub fn markdown_edit(source: Signal<String>) -> NodeBuilder {
    use super::splitter::{SplitterOrientation, splitter};
    use super::text_edit::text_edit;

    container()
        .layout(LayoutStyle::flex_row().fill_width().grow(1.0))
        .child(
            splitter(SplitterOrientation::Horizontal)
                .child(text_edit(source), 360.0)
                .child(markdown(source), 360.0),
        )
}

fn block_node(block: Block) -> NodeBuilder {
    match block {
        Block::Heading { level, text: body } => {
            let class = match level {
                1 => "text-xl font-bold",
                2 => "text-lg font-bold",
                _ => "text-base font-bold",
            };
            rich_line(&body, class)
        }
        Block::Paragraph(body) => rich_line(&body, "text-on-surface"),
        Block::List(items) => {
            let mut list = container().layout(
                LayoutStyle::flex_column()
                    .fill_width()
                    .gap(4.0)
                    .padding_left(12.0),
            );
            for item in items {
                list = list.child(rich_line(&format!("• {item}"), "text-on-surface"));
            }
            list
        }
        Block::Code(body) => container()
            .class("bg-surface rounded-md font-mono text-sm")
            .layout(LayoutStyle::block().fill_width().padding(8.0))
            .child(text(body)),
        Block::Mermaid(body) => super::mermaid::mermaid(body),
        Block::Table(rows) => table_node(rows),
    }
}

fn table_node(rows: Vec<Vec<String>>) -> NodeBuilder {
    let mut table = container()
        .layout(LayoutStyle::flex_column().fill_width())
        .class("border border-gray-6 rounded-md");
    for (i, row) in rows.into_iter().enumerate() {
        let mut line = container()
            .layout(LayoutStyle::flex_row().fill_width())
            .class(if i == 0 {
                "border-b border-gray-6 font-bold"
            } else {
                "border-b border-gray-6"
            });
        for cell in row {
            line = line.child(
                container()
                    .layout(LayoutStyle::block().grow(1.0).padding(6.0))
                    .child(rich_line(&cell, "text-on-surface")),
            );
        }
        table = table.child(line);
    }
    table
}

fn rich_line(source: &str, class: &'static str) -> NodeBuilder {
    let spans = parse_inlines(source);
    if let [Span::Text(body)] = spans.as_slice() {
        return container().child(text(body.clone()).class(class));
    }
    let mut row = container().layout(
        LayoutStyle::flex_row()
            .flex_wrap(FlexWrap::Wrap)
            .fill_width()
            .gap(0.0),
    );
    for span in spans {
        row = row.child(span_text(span, class));
    }
    row
}

fn span_text(span: Span, class: &'static str) -> impl sigil_kernel::IntoNode + 'static {
    match span {
        Span::Text(body) | Span::Italic(body) => text(body).class(class),
        Span::Bold(body) => text(body).class("font-bold"),
        Span::Code(body) => text(body).class("font-mono text-sm"),
        Span::Link { text: body, href } => text(body).class("text-accent-11").description(href),
    }
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
