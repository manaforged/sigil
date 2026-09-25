use std::ops::Range;

use crate::kernel::{NodeBuilder, Role, container};
use crate::layout::prelude::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle};
use crate::style::{BORDER_WIDTH, NodeStyleExt, Style, Token};

use super::GUTTER_W;
use super::patch::{Kind, Pair, changed};
use crate::app::cell_width;
use crate::widgets::shared::label;

const fn tones(mark: char) -> (Option<Token>, Token, Option<Token>) {
    match mark {
        '-' => (Some(Token::Error3), Token::Error11, Some(Token::Error5)),
        '+' => (
            Some(Token::Success3),
            Token::Success11,
            Some(Token::Success5),
        ),
        _ => (None, Token::Gray11, None),
    }
}

const fn line_state(mark: char) -> &'static str {
    match mark {
        '-' => "removed",
        '+' => "added",
        _ => "unchanged",
    }
}

pub(super) fn segments(
    body: &str,
    range: Option<Range<usize>>,
    fg: Token,
    hit: Option<Token>,
) -> Vec<NodeBuilder> {
    let body = body.replace('\t', "    ");
    let Some(range) = range.filter(|range| range.start < range.end && range.end <= body.len())
    else {
        return vec![container().child(label(body).class("text-xs font-mono").class(fg.text()))];
    };
    let mut parts = Vec::new();
    for (slice, mark) in [
        (&body[..range.start], false),
        (&body[range.clone()], true),
        (&body[range.end..], false),
    ] {
        if slice.is_empty() {
            continue;
        }
        let node = label(slice.to_string())
            .class("text-xs font-mono")
            .class(fg.text());
        parts.push(container().child(match hit.filter(|_| mark) {
            Some(hit) => node.class(hit.bg()),
            None => node,
        }));
    }
    parts
}

pub(super) fn source_row(
    body: Option<&str>,
    number: usize,
    mark: char,
    range: Option<Range<usize>>,
) -> NodeBuilder {
    source_row_ruled(body, number, mark, range, false)
}

fn source_row_ruled(
    body: Option<&str>,
    number: usize,
    mark: char,
    range: Option<Range<usize>>,
    ruled: bool,
) -> NodeBuilder {
    let (bg, fg, hit) = tones(mark);
    let rule = |node: NodeBuilder| {
        if ruled {
            node.style(
                Style::new()
                    .border_right(BORDER_WIDTH.0)
                    .border_theme(Token::Border.name()),
            )
        } else {
            node
        }
    };
    let row = rule(
        container().layout(
            LayoutStyle::flex_row()
                .fill_width()
                .padding_x(super::ROW_INSET)
                .align_items(AlignItems::Center)
                .gap(0.0),
        ),
    );
    let row = match bg {
        Some(bg) => row.class(bg.bg()),
        None => row,
    };
    let Some(body) = body else {
        return rule(container().layout(LayoutStyle::flex_row().fill_width().gap(0.0)));
    };
    let row = row
        .role(Role::ListItem)
        .description(line_state(mark))
        .child(
            label(format!("{number:>4} {mark} "))
                .layout(LayoutStyle::block().width(GUTTER_W))
                .class("text-xs font-mono")
                .class(Token::Gray9.text()),
        );
    segments(body, range, fg, hit)
        .into_iter()
        .fold(row, NodeBuilder::child)
}

pub(super) fn weights(pairs: &[Pair]) -> (f32, f32) {
    let widest = |side: fn(&Pair) -> Option<&str>| {
        pairs
            .iter()
            .filter(|pair| matches!(pair.kind, Kind::Context | Kind::Change))
            .filter_map(side)
            .map(|line| cell_width(&line.replace('\t', "    ")))
            .max()
            .unwrap_or(0)
    };
    let weight =
        |cells: usize| f32::from(u16::try_from(cells).unwrap_or(u16::MAX)) + GUTTER_W / 8.0;
    (
        weight(widest(|pair| pair.left.as_deref())),
        weight(widest(|pair| pair.right.as_deref())),
    )
}

fn half(
    body: Option<&str>,
    (number, mark): (usize, char),
    range: Option<Range<usize>>,
    ruled: bool,
    grow: f32,
) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .grow(grow)
                .flex_basis(Dimension::Px(0.0))
                .min_size(0.0, 0.0),
        )
        .child(source_row_ruled(body, number, mark, range, ruled))
}

pub(super) fn split_row(pair: &Pair, old: usize, new: usize, (lw, rw): (f32, f32)) -> NodeBuilder {
    let left = pair.left.as_deref();
    let right = pair.right.as_deref();
    let ranges =
        (pair.kind == Kind::Change).then(|| changed(left.unwrap_or(""), right.unwrap_or("")));
    let (left_mark, right_mark) = if ranges.is_some() {
        ('-', '+')
    } else {
        (' ', ' ')
    };
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .min_size(0.0, 0.0)
                .gap(0.0),
        )
        .child(half(
            left,
            (old, left_mark),
            ranges.as_ref().map(|r| r.0.clone()),
            true,
            lw,
        ))
        .child(half(
            right,
            (new, right_mark),
            ranges.map(|r| r.1),
            false,
            rw,
        ))
}

pub(super) fn unified_rows(group: &[Pair], old: usize, new: usize) -> Vec<NodeBuilder> {
    let mut rows = Vec::new();
    for (added, mut number) in [(false, old), (true, new)] {
        for pair in group {
            let left = pair.left.as_deref();
            let right = pair.right.as_deref();
            let (a, b) = changed(left.unwrap_or(""), right.unwrap_or(""));
            let (body, range, mark) = if added {
                (right, b, '+')
            } else {
                (left, a, '-')
            };
            if let Some(body) = body {
                rows.push(source_row(Some(body), number, mark, Some(range)));
                number += 1;
            }
        }
    }
    rows
}
