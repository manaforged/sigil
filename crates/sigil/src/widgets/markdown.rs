mod chart_parse;
mod containers;
mod inline;
pub(crate) mod parse;
mod render;
mod stream;
mod table;

use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, Role, TextBuilder, container, text};
use crate::layout::cols;
use crate::layout::snap::CellForm;
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::reactive::Reactive;
use crate::reactive::{Signal, derived};
use crate::style::{NodeStyleExt, Radius, Token, space};

use super::layout_ext::WidgetLayoutExt;
use std::rc::Rc;

use parse::Block;
use render::Tone;

#[must_use]
pub fn markdown(source: Signal<String>) -> NodeBuilder {
    let mut stream = stream::Stream::default();
    let blocks = derived(move || source.with(|body| stream.update(body)));
    frame().children_keyed(move || {
        blocks
            .get()
            .iter()
            .map(|(key, block)| (*key, KeyedBlock(Rc::clone(block))))
            .collect::<Vec<_>>()
    })
}

#[must_use]
pub fn markdown_inline(source: &str) -> TextBuilder {
    let body = inline::styled(source);
    text(body.text).runs(body.runs)
}

struct KeyedBlock(Rc<Block>);

impl IntoNode for KeyedBlock {
    fn build(self, cx: &mut Cx) -> NodeId {
        block_node(Rc::unwrap_or_clone(self.0)).build(cx)
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
            .gap(space(2)),
    )
}

pub struct MarkdownEditBuilder {
    source: Signal<String>,
    disabled: Option<Signal<bool>>,
}

#[must_use]
pub const fn markdown_edit(source: Signal<String>) -> MarkdownEditBuilder {
    MarkdownEditBuilder {
        source,
        disabled: None,
    }
}

impl MarkdownEditBuilder {
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        use super::splitter::{SplitterOrientation, splitter};
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
                splitter(SplitterOrientation::Horizontal)
                    .child(editor, 360.0)
                    .pane(markdown(self.source))
                    .initial(360.0)
                    .grow(fit)
                    .commit()
                    .grow(fit),
            )
    }
}

impl IntoNode for MarkdownEditBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}

fn block_node(block: Block) -> NodeBuilder {
    styled_block(block, Tone(Token::OnSurface))
}

fn styled_block(block: Block, tone: Tone) -> NodeBuilder {
    match block {
        Block::Heading { level, text: body } => render::heading(level, &body),
        Block::Paragraph(body) => render::rich(&body, "", tone, Role::Paragraph),
        Block::Quote(blocks) => render::quote(blocks, styled_block),
        Block::List(list) => render::list(list, tone, styled_block),
        Block::Code { lang, body } => render::code(&lang, &body),
        Block::Mermaid(body) => super::mermaid::mermaid(body),
        Block::Chart { title, rows } => chart_node(title, &rows),
        Block::Image { alt, src } => image_node(&alt, &src),
        Block::Table(rows) => table::table(&rows),
        Block::Rule => render::rule(),
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
                .padding(space(2))
                .gap(space(1)),
        )
        .class([Token::Field.bg(), Radius::Md.atom()]);
    if let Some(title) = title {
        column = column.child(text(title).class("text-xs").class(Token::Gray11.text()));
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
    container().child(text(format!("[{alt}]")).class(Token::Gray11.text()))
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
    let size = (width * scale, (height * scale).min(384.0));
    Some(CellForm::new(size, (size.0, size.1 * 0.5)).pick())
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
