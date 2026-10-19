use std::ops::Range;

use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle, Overflow};
use crate::style::{BORDER_WIDTH, Paint, Token, space};
use crate::widgets::highlight::{highlight, text_runs};

use super::model::{Cell, DiffRow};
use super::patch::LineKind;
use super::selection::Picker;

#[derive(Clone, Copy)]
pub(super) struct Draw {
    pub(super) picker: Option<Picker>,
    pub(super) wrap: bool,
}

pub(super) const GUTTER_W: f32 = 56.0;
pub(super) const ROW_INSET: f32 = space(2);

const fn look(kind: LineKind) -> (Option<Token>, Token, &'static str, char) {
    match kind {
        LineKind::Removed => (Some(Token::Error3), Token::Error11, "removed", '-'),
        LineKind::Added => (Some(Token::Success3), Token::Success11, "added", '+'),
        LineKind::Context | LineKind::Note => (None, Token::Gray11, "unchanged", ' '),
    }
}

pub(super) fn render(row: &DiffRow, draw: Draw) -> NodeBuilder {
    let picker = draw.picker;
    match row {
        DiffRow::Meta(body) => text_row(body.clone(), "text-xs font-mono", Token::Gray10),
        DiffRow::Hunk { hunk, header } => {
            let line = text_row(header.clone(), "text-xs font-mono", Token::Info11);
            match picker {
                Some(picker) => picker.hunk(line, *hunk),
                None => line,
            }
        }
        DiffRow::Split {
            left,
            right,
            weights,
        } => split_row(left.as_ref(), right.as_ref(), *weights, draw),
        DiffRow::Unified(cell) => source_row(cell, draw),
        DiffRow::Overflow => text_row(
            "… expand tool output to see the remaining changes".to_string(),
            "text-xs",
            Token::Gray10,
        ),
    }
}

pub(super) fn text_row(body: String, class: &'static str, tone: Token) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().fill_width().padding_x(ROW_INSET))
        .child(
            text(body)
                .nowrap()
                .layout(LayoutStyle::block().fill_width().min_size(0.0, 0.0))
                .class(class)
                .class(tone.text()),
        )
}

fn segments(cell: &Cell, fg: Token, wrap: bool) -> Vec<NodeBuilder> {
    let marks = cell
        .language
        .and_then(|language| highlight(language, &cell.body).into_iter().next())
        .unwrap_or_default();
    let len = cell.body.len();
    if wrap {
        return vec![
            text(cell.body.clone())
                .wrap()
                .runs(text_runs(&marks, 0..len))
                .class("text-xs font-mono")
                .class(fg.text())
                .layout(LayoutStyle::block().grow(1.0).min_size(0.0, 0.0))
                .into(),
        ];
    }
    let part = |range: Range<usize>| {
        let slice = cell.body.get(range.clone()).unwrap_or_default().to_string();
        let last = range.end == len;
        let body = text(slice)
            .nowrap()
            .runs(text_runs(&marks, range))
            .class("text-xs font-mono")
            .class(fg.text());
        if last {
            container()
                .layout(LayoutStyle::block().min_size(0.0, 0.0))
                .child(body.ellipsize())
        } else {
            container()
                .layout(LayoutStyle::block().shrink(0.0))
                .child(body)
        }
    };
    let Some(range) = cell
        .range
        .clone()
        .filter(|range| range.start < range.end && range.end <= len)
    else {
        return vec![part(0..len)];
    };
    [0..range.start, range.clone(), range.end..len]
        .into_iter()
        .filter(|slice| !slice.is_empty())
        .map(part)
        .collect()
}

pub(super) fn source_row(cell: &Cell, draw: Draw) -> NodeBuilder {
    let (bg, fg, state, mark) = look(cell.kind);
    let row = container().layout(
        LayoutStyle::flex_row()
            .fill_width()
            .min_size(0.0, 0.0)
            .padding_x(ROW_INSET)
            .align_items(AlignItems::Center)
            .gap(0.0),
    );
    let row = match bg {
        Some(bg) => row.class(bg.bg()),
        None => row,
    };
    let row = row.role(Role::ListItem).description(state).child(
        text(format!("{:>4} {} ", cell.number, mark))
            .nowrap()
            .layout(LayoutStyle::block().width(GUTTER_W).shrink(0.0))
            .class("text-xs font-mono")
            .class(Token::Gray9.text()),
    );
    let row = segments(cell, fg, draw.wrap)
        .into_iter()
        .fold(row, NodeBuilder::child);
    match draw.picker.filter(|_| cell.kind.is_change()) {
        Some(picker) => picker.line(row, cell.id),
        None => row,
    }
}

fn half(cell: Option<&Cell>, draw: Draw, ruled: bool, grow: f32) -> NodeBuilder {
    let column = container().layout(
        LayoutStyle::flex_column()
            .grow(grow)
            .flex_basis(Dimension::Px(0.0))
            .min_size(0.0, 0.0)
            .overflow_x(Overflow::Hidden),
    );
    let column = if ruled {
        column
            .border_right(BORDER_WIDTH.0)
            .border_color(Token::Border)
    } else {
        column
    };
    column.child(cell.map_or_else(
        || container().layout(LayoutStyle::flex_row().fill_width().gap(0.0)),
        |cell| source_row(cell, draw),
    ))
}

fn split_row(
    left: Option<&Cell>,
    right: Option<&Cell>,
    (lw, rw): (f32, f32),
    draw: Draw,
) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .min_size(0.0, 0.0)
                .gap(0.0),
        )
        .child(half(left, draw, true, lw))
        .child(half(right, draw, false, rw))
}
