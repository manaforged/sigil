mod blocks;
mod chart_parse;
mod containers;
mod details;
mod events;
mod inline;
mod latex;
pub(crate) mod parse;
mod refs;
mod render;
mod repair;
mod spans;
mod stream;
mod table;
mod table_data;

use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, TextBuilder, container, text};
use crate::layout::LayoutStyle;
use crate::layout::Space;
use crate::layout::cols;
use crate::layout::snap::CellForm;
use crate::reactive::Reactive;
use crate::reactive::{Signal, derived};
use crate::style::{Radius, Token, Tone as ChartTone, space};
use crate::widgets::Widget;
use std::rc::Rc;

use parse::Block;
use render::TokenTone;

/// Rendered markdown.
#[must_use]
pub fn markdown(source: Signal<String>) -> NodeBuilder {
    let mut stream = stream::Stream::default();
    let blocks = derived(move || source.with(|body| stream.update(body)));
    frame().children_keyed(move || {
        blocks
            .get()
            .iter()
            .enumerate()
            .map(|(at, (key, block))| {
                let lead = at > 0 && matches!(**block, Block::Heading { .. });
                (*key, KeyedBlock(Rc::clone(block), lead))
            })
            .collect::<Vec<_>>()
    })
}

/// One wrapped paragraph with inline markdown spans.
#[must_use]
pub fn markdown_inline(source: &str) -> TextBuilder {
    let body = inline::styled(&parse::parse_inlines(source));
    text(body.text).runs(body.runs)
}

struct KeyedBlock(Rc<Block>, bool);

impl IntoNode for KeyedBlock {
    fn build(self, cx: &mut Cx) -> NodeId {
        let node = block_node(Rc::unwrap_or_clone(self.0));
        if !self.1 {
            return node.build(cx);
        }
        container()
            .layout(
                LayoutStyle::block()
                    .fill_width()
                    .padding_top(Space::px(CellForm::new(space(2), 0.0).pick())),
            )
            .child(node)
            .build(cx)
    }
}

pub(crate) fn document(blocks: Signal<Vec<Block>>) -> NodeBuilder {
    frame().children_reactive(move || blocks.get().into_iter().map(block_node).collect::<Vec<_>>())
}

fn frame() -> NodeBuilder {
    container().role(Role::Document).layout(
        LayoutStyle::flex_column()
            .fill_width()
            .padding_right(cols(2.0))
            .gap(Space::Sm),
    )
}

/// Builder of a markdown editor.
#[derive(Debug)]
pub struct MarkdownEditBuilder {
    source: Signal<String>,
    disabled: Option<Signal<bool>>,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

/// A markdown editor with preview.
#[must_use]
pub const fn markdown_edit(source: Signal<String>) -> MarkdownEditBuilder {
    MarkdownEditBuilder {
        source,
        disabled: None,
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl MarkdownEditBuilder {
    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    fn root(self) -> NodeBuilder {
        use super::orientation::Orientation;
        use super::splitter::splitter;
        use super::text_edit::text_edit;

        let editor = text_edit(self.source);
        let editor = match self.disabled {
            Some(disabled) => editor.disabled(disabled),
            None => editor,
        };
        let fit = CellForm::new(0.0, 1.0).pick();
        container()
            .layout(LayoutStyle::flex_row().fill_width().grow(1.0))
            .child(
                splitter(Orientation::Horizontal)
                    .child(editor, 360.0)
                    .pane(markdown(self.source))
                    .width(360.0)
                    .grow(fit)
                    .commit()
                    .grow(fit),
            )
    }
}

builder_impls!([] MarkdownEditBuilder: widget, node);

fn block_node(block: Block) -> NodeBuilder {
    styled_block(block, TokenTone(Token::OnSurface))
}

fn styled_block(block: Block, tone: TokenTone) -> NodeBuilder {
    match block {
        Block::Heading { level, body } => render::heading(level, &body),
        Block::Paragraph(body) => render::rich(&body, "", tone, Role::Paragraph),
        Block::Quote(blocks) => render::quote(blocks, styled_block),
        Block::Alert { kind, blocks } => blocks::alert(kind, blocks, styled_block),
        Block::Details { summary, blocks } => blocks::details(summary, blocks, styled_block),
        Block::List(list) => render::list(list, tone, styled_block),
        Block::Code { lang, body } => render::code(&lang, &body),
        Block::Math(source) => blocks::math(&source),
        Block::Mermaid(body) => super::mermaid::mermaid(body),
        Block::VegaLite(source) => super::vega::figure(&source, vega_fallback),
        Block::Graphviz(source) => super::graphviz::figure(&source),
        Block::Chart(data) => chart_node(data),
        Block::Image { alt, src } => image_node(&alt, &src),
        Block::Table(data) => table::table(data),
        Block::Footnotes(notes) => blocks::footnotes(notes, styled_block),
        Block::Html(html) => {
            let body = [parse::Inline::Text(html.trim_end().to_string())];
            render::rich(&body, "", tone, Role::Paragraph)
        }
        Block::Rule => render::rule(),
    }
}

fn vega_fallback(source: &str) -> NodeBuilder {
    super::chart::vega_lite(source).unwrap_or_else(|| render::code("json", source))
}

fn chart_node(data: chart_parse::ChartData) -> NodeBuilder {
    let mut column = container()
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding(Space::Sm)
                .gap(Space::Xs),
        )
        .class([Token::Field.background(), Radius::Md.atom()]);
    if let Some(title) = data.title {
        column = column.child(text(title).class("text-xs").class(Token::Gray11.text()));
    }
    let mut chart = super::chart::chart(data.values).labels(data.labels);
    if data.line {
        chart = chart.line();
    }
    for values in data.series {
        chart = chart.series("", values, ChartTone::Primary);
    }
    column.child(chart)
}

fn image_node(alt: &str, src: &str) -> NodeBuilder {
    #[cfg(feature = "image-decode")]
    if let Some(source) = crate::kernel::ImageSource::from_data_uri(src)
        && let Some((width, height)) = display_size(source.width(), source.height())
    {
        return container().child(
            crate::kernel::image(source)
                .alt(alt.to_string())
                .fit(crate::kernel::ObjectFit::Contain)
                .layout(LayoutStyle::block().width(width).height(height)),
        );
    }
    let _ = src;
    container().child(text(format!("[{alt}]")).class(Token::Gray11.text()))
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
    let size = (width * scale, (height * scale).min(384.0));
    Some(CellForm::new(size, (size.0, size.1 * 0.5)).pick())
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
