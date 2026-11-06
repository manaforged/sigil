use std::rc::Rc;

use pulldown_cmark::BlockQuoteKind;

use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, LayoutStyle, cols};
use crate::reactive::signal;
use crate::style::{Radius, StyleAtom, Token, space};
use crate::widgets::Widget;

use super::inline::note_mark;
use super::parse::{Block, Inline, Note};
use super::render::{TokenTone, rich, stack, tinted};

pub(super) type BlockNode = fn(Block, TokenTone) -> NodeBuilder;

struct Callout {
    title: &'static str,
    tint: Token,
    edge: Token,
    ink: Token,
}

const fn callout(kind: BlockQuoteKind) -> Callout {
    let (title, tint, edge, ink) = match kind {
        BlockQuoteKind::Note => ("Note", Token::Info3, Token::Info8, Token::Info11),
        BlockQuoteKind::Tip => ("Tip", Token::Success3, Token::Success8, Token::Success11),
        BlockQuoteKind::Important => ("Important", Token::Accent3, Token::Accent8, Token::Accent11),
        BlockQuoteKind::Warning => ("Warning", Token::Warn3, Token::Warn8, Token::Warn11),
        BlockQuoteKind::Caution => ("Caution", Token::Error3, Token::Error8, Token::Error11),
    };
    Callout {
        title,
        tint,
        edge,
        ink,
    }
}

pub(super) fn alert(kind: BlockQuoteKind, blocks: Vec<Block>, node: BlockNode) -> NodeBuilder {
    let style = callout(kind);
    let pad = CellForm::new(space(2), 0.0).pick();
    container()
        .role(Role::Note)
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding_x(cols(1.0))
                .padding_y(pad)
                .gap(space(1)),
        )
        .class([
            style.tint.bg(),
            StyleAtom::BorderLeft(3),
            style.edge.border(),
            Radius::Md.atom(),
        ])
        .child(text(style.title).class("font-bold").class(style.ink.text()))
        .child(stack(blocks, (TokenTone(Token::OnSurface), space(2)), node))
}

pub(super) fn details(summary: Vec<Inline>, blocks: Vec<Block>, node: BlockNode) -> NodeBuilder {
    let blocks = Rc::new(blocks);
    let tone = TokenTone(Token::OnSurface);
    container()
        .role(Role::Details)
        .layout(LayoutStyle::block().fill_width())
        .child(crate::widgets::disclosure(
            signal(false),
            move |_| rich(&summary, "font-bold", tone, Role::Paragraph),
            move || {
                container()
                    .layout(LayoutStyle::block().fill_width().padding_left(cols(2.0)))
                    .child(stack((*blocks).clone(), (tone, space(2)), node))
            },
        ))
}

#[expect(clippy::cast_precision_loss, reason = "marker widths are a few columns")]
pub(super) fn footnotes(notes: Vec<Note>, node: BlockNode) -> NodeBuilder {
    let tone = TokenTone(Token::OnSurfaceMuted);
    let mut column = container()
        .role(Role::List)
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding_top(CellForm::new(space(2), 0.0).pick())
                .gap(space(1)),
        )
        .class("text-sm")
        .class([StyleAtom::BorderTop(1), Token::Border.border()]);
    let width = notes
        .iter()
        .map(|note| note_mark(&note.label).chars().count())
        .max()
        .unwrap_or(1)
        .min(12)
        + 2;
    for note in notes {
        let mark = tinted(text(note_mark(&note.label)).nowrap(), TokenTone(Token::Accent11))
            .layout(LayoutStyle::block().width(cols(width as f32)).shrink(0.0));
        let row = container()
            .role(Role::ListItem)
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .align_items(AlignItems::Start),
            )
            .child(mark)
            .child(stack(note.blocks, (tone, space(1)), node));
        column = column.child(row);
    }
    column
}

pub(super) fn math(source: &str) -> NodeBuilder {
    container()
        .role(Role::Math)
        .layout(LayoutStyle::block().fill_width())
        .child(crate::widgets::math::block(source))
}
