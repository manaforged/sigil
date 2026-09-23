use std::ops::Range;

use crate::kernel::{NodeBuilder, container};
use crate::layout::prelude::{AlignItems, Dimension, LayoutBuilderExt, LayoutStyle};
use crate::style::{NodeStyleExt, Style};

use super::GUTTER_W;
use super::patch::{Kind, Pair, changed};
use crate::widgets::shared::label;

const fn tones(mark: char) -> (&'static str, &'static str, &'static str) {
    match mark {
        '-' => ("bg-error-3", "text-error-11", "bg-error-5"),
        '+' => ("bg-success-3", "text-success-11", "bg-success-5"),
        _ => ("", "text-gray-11", ""),
    }
}

pub(super) fn segments(
    body: &str,
    range: Option<Range<usize>>,
    fg: &'static str,
    hit: &'static str,
) -> Vec<NodeBuilder> {
    let body = body.replace('\t', "    ");
    let Some(range) = range.filter(|range| range.start < range.end && range.end <= body.len())
    else {
        return vec![container().child(label(body).class("text-xs font-mono").class(fg))];
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
            .class(fg);
        parts.push(container().child(if mark { node.class(hit) } else { node }));
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
            node.style(Style::new().border_right(1).border_theme("border"))
        } else {
            node
        }
    };
    let row = rule(
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .align_items(AlignItems::Center)
                    .gap(0.0),
            )
            .class(bg),
    );
    let Some(body) = body else {
        return rule(container().layout(LayoutStyle::flex_row().fill_width().gap(0.0)));
    };
    let row = row.child(
        label(format!("{number:>4} {mark} "))
            .layout(LayoutStyle::block().width(GUTTER_W))
            .class("text-xs font-mono text-gray-9"),
    );
    segments(body, range, fg, hit)
        .into_iter()
        .fold(row, NodeBuilder::child)
}

fn half(
    body: Option<&str>,
    number: usize,
    mark: char,
    range: Option<Range<usize>>,
    ruled: bool,
) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .grow(1.0)
                .flex_basis(Dimension::Px(0.0))
                .min_size(0.0, 0.0),
        )
        .child(source_row_ruled(body, number, mark, range, ruled))
}

pub(super) fn split_row(pair: &Pair, old: usize, new: usize) -> NodeBuilder {
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
            old,
            left_mark,
            ranges.as_ref().map(|r| r.0.clone()),
            true,
        ))
        .child(half(right, new, right_mark, ranges.map(|r| r.1), false))
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
