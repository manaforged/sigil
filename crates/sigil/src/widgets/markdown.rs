pub(crate) mod parse;

use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::FlexWrap;
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived_eq};
use crate::style::NodeStyleExt;

#[cfg(feature = "image-decode")]
use super::layout_ext::WidgetLayoutExt;
use std::fmt::Write;

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
            .class("bg-field rounded-md font-mono text-sm")
            .layout(LayoutStyle::block().fill_width().padding(8.0))
            .child(text(body)),
        Block::Mermaid(body) => super::mermaid::mermaid(body),
        Block::Chart { title, rows } => chart_node(title, &rows),
        Block::Image { alt, src } => image_node(&alt, &src),
        Block::Table(rows) => table_node(&rows),
    }
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "chart labels are display values"
)]
fn chart_node(title: Option<String>, rows: &[(String, f64)]) -> NodeBuilder {
    let mut column = container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding(8.0)
                .gap(4.0),
        )
        .class("bg-field rounded-md");
    if let Some(title) = title {
        column = column.child(text(title).class("text-xs text-gray-11"));
    }
    column.child(
        super::chart::chart(rows.iter().map(|(_, value)| *value as f32))
            .labels(rows.iter().map(|(name, _)| name.clone()))
            .height(8.0 * (rows.len() as f32 + 3.0))
            .width(480.0),
    )
}

fn image_node(alt: &str, src: &str) -> NodeBuilder {
    #[cfg(feature = "image-decode")]
    if let Some(decoded) = decode_src(src)
        && let Some((width, height)) = display_size(decoded.width(), decoded.height())
        && let Some(image) =
            super::image::image_rgba(decoded.width(), decoded.height(), decoded.into_raw())
    {
        return container().child(
            image
                .alt(alt.to_string())
                .fit(crate::kernel::ObjectFit::Contain)
                .layout(LayoutStyle::block().width(width).height(height)),
        );
    }
    let _ = src;
    container().child(text(format!("[{alt}]")).class("text-gray-11"))
}

#[cfg(feature = "image-decode")]
fn decode_src(src: &str) -> Option<::image::RgbaImage> {
    use base64::Engine;
    let (meta, payload) = src.strip_prefix("data:")?.split_once(',')?;
    if !meta.contains("base64") || meta.contains("svg") {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()?;
    Some(::image::load_from_memory(&bytes).ok()?.into_rgba8())
}

#[cfg(feature = "image-decode")]
#[expect(clippy::cast_precision_loss, reason = "image dimensions fit f32")]
fn display_size(width: u32, height: u32) -> Option<(f32, f32)> {
    if width == 0 || height == 0 {
        return None;
    }
    let width = width as f32;
    let height = height as f32;
    let scale = (480.0 / width).min(1.0);
    Some((width * scale, (height * scale).min(384.0)))
}

fn table_node(rows: &[Vec<String>]) -> NodeBuilder {
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![1usize; cols];
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index) {
                *width = (*width).max(cell.chars().count().max(1));
            }
        }
    }
    let rule = |left: char, mid: char, right: char| {
        let mut line = String::new();
        line.push(left);
        for (index, width) in widths.iter().enumerate() {
            line.push_str(&"─".repeat(*width + 2));
            line.push(if index + 1 == cols { right } else { mid });
        }
        line
    };
    let mut table = container()
        .layout(LayoutStyle::flex_column().fill_width().gap(0.0))
        .class("bg-field font-mono text-sm");
    table = table.child(text(rule('┌', '┬', '┐')).class("text-gray-10"));
    for (index, row) in rows.iter().enumerate() {
        let mut line = String::from("│");
        for (cell_index, width) in widths.iter().enumerate() {
            let cell = row.get(cell_index).map_or("", String::as_str);
            let pad = width.saturating_sub(cell.chars().count());
            let _ = write!(line, " {cell}{} │", " ".repeat(pad));
        }
        let class = if index == 0 {
            "font-bold text-on-surface"
        } else {
            "text-on-surface"
        };
        table = table.child(text(line).class(class));
        if index == 0 && rows.len() > 1 {
            table = table.child(text(rule('├', '┼', '┤')).class("text-gray-10"));
        }
    }
    table.child(text(rule('└', '┴', '┘')).class("text-gray-10"))
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

fn span_text(span: Span, class: &'static str) -> impl crate::kernel::IntoNode + 'static {
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
