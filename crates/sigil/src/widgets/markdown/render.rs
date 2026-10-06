use crate::kernel::{NodeBuilder, Role, TextBuilder, container, text};
use crate::layout::cols;
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::signal;
use crate::style::{Px, Radius, StyleAtom, TextFace, TextRun, Token, space};
use crate::widgets::interaction::InteractionState;
use crate::widgets::{Highlight, highlight};

use super::super::checkbox::{CheckMark, checkbox_indicator};
use super::containers::{List, Task};
use super::inline::styled;
use super::parse::Block;

#[derive(Clone, Copy)]
pub(super) struct Tone(pub(super) Token);

pub(super) fn tinted(node: TextBuilder, tone: Tone) -> TextBuilder {
    node.class(tone.0.text())
}

pub(super) fn rich(source: &str, class: &'static str, tone: Tone, role: Role) -> NodeBuilder {
    let body = styled(source);
    container()
        .role(role)
        .layout(LayoutStyle::block().fill_width())
        .child(tinted(text(body.text).runs(body.runs).class(class), tone))
}

pub(super) fn heading(level: u8, body: &str) -> NodeBuilder {
    let class = match level {
        1 => CellForm::new("text-xl font-bold", "font-bold underline").pick(),
        2 => "text-lg font-bold",
        _ => "text-base font-bold",
    };
    let grid = match level {
        1 | 2 => Token::Accent11,
        3 => Token::OnSurface,
        _ => Token::OnSurfaceMuted,
    };
    let tone = Tone(CellForm::new(Token::OnSurface, grid).pick());
    let level = usize::from(level);
    rich(body, class, tone, Role::Heading).mutate(move |node| node.a11y.level = Some(level))
}

pub(super) fn rule() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fill_width().height(1.0).shrink(0.0))
        .class(Token::BorderStrong.bg())
}

pub(super) fn stack(
    blocks: Vec<Block>,
    (tone, gap): (Tone, f32),
    node: impl Fn(Block, Tone) -> NodeBuilder,
) -> NodeBuilder {
    let mut column = container().layout(
        LayoutStyle::flex_column()
            .grow(1.0)
            .shrink(1.0)
            .min_width(0.0)
            .gap(gap),
    );
    for block in blocks {
        column = column.child(node(block, tone));
    }
    column
}

pub(super) fn quote(blocks: Vec<Block>, node: impl Fn(Block, Tone) -> NodeBuilder) -> NodeBuilder {
    container()
        .role(Role::Blockquote)
        .layout(LayoutStyle::flex_row().fill_width().padding_left(cols(1.0)))
        .class([StyleAtom::BorderLeft(Px(2)), Token::BorderStrong.border()])
        .child(stack(blocks, (Tone(Token::OnSurfaceMuted), space(2)), node))
}

fn marker(start: Option<u64>, index: usize) -> String {
    start.map_or_else(
        || String::from("•"),
        |start| format!("{}.", start.saturating_add(index as u64)),
    )
}

#[expect(
    clippy::cast_precision_loss,
    reason = "marker widths are a few columns"
)]
pub(super) fn list(
    list: List,
    tone: Tone,
    node: impl Fn(Block, Tone) -> NodeBuilder + Copy,
) -> NodeBuilder {
    let start = list.start;
    let width = (0..list.items.len())
        .map(|index| marker(start, index).chars().count())
        .max()
        .unwrap_or(1)
        + 1;
    let mut column = container()
        .role(Role::List)
        .layout(LayoutStyle::flex_column().fill_width().gap(space(1)));
    for (index, item) in list.items.into_iter().enumerate() {
        let row = container().role(Role::ListItem).layout(
            LayoutStyle::flex_row()
                .fill_width()
                .align_items(AlignItems::Start),
        );
        let row = match item.task {
            Some(task) => row.child(task_mark(task)),
            None => row.child(
                tinted(text(marker(start, index)).nowrap(), tone)
                    .layout(LayoutStyle::block().width(cols(width as f32)).shrink(0.0)),
            ),
        };
        column = column.child(row.child(stack(item.blocks, (tone, space(1)), node)));
    }
    column
}

fn task_mark(task: Task) -> NodeBuilder {
    let checked = signal(task == Task::Done);
    container()
        .role(Role::CheckBox)
        .checked(checked)
        .layout(LayoutStyle::block().width(cols(4.0)).shrink(0.0))
        .child(checkbox_indicator(
            CheckMark {
                checked,
                partial: signal(false),
            },
            None,
            crate::style::Tone::Neutral,
            InteractionState::new(true),
        ))
}

pub(super) fn code(lang: &str, body: &str) -> NodeBuilder {
    let pad = CellForm::new(space(2), 0.0).pick();
    let mut block = container()
        .role(Role::Code)
        .class("font-mono text-sm")
        .class([Token::Field.bg(), Radius::Md.atom()])
        .layout(
            LayoutStyle::flex_column()
                .fill_width()
                .padding_x(space(2))
                .padding_y(pad),
        );
    for (line, marks) in body.split('\n').zip(highlight(lang, body)) {
        let rest = line.trim_start();
        let lead = &line[..line.len() - rest.len()];
        let mut row = container().layout(LayoutStyle::flex_row().fill_width());
        if !lead.is_empty() {
            row = row.child(
                text(lead.to_string())
                    .nowrap()
                    .layout(LayoutStyle::block().shrink(0.0)),
            );
        }
        let rest = if rest.is_empty() { " " } else { rest };
        row = row.child(
            text(rest.to_string())
                .runs(code_runs(&marks, lead.len()))
                .class(Token::OnSurface.text())
                .layout(LayoutStyle::block().grow(1.0).shrink(1.0).min_width(0.0)),
        );
        block = block.child(row);
    }
    block
}

fn code_runs(marks: &[Highlight], lead: usize) -> Vec<TextRun> {
    marks
        .iter()
        .filter(|mark| mark.start >= lead)
        .map(|mark| TextRun {
            start: mark.start - lead,
            end: mark.end - lead,
            face: TextFace {
                color: Some(mark.token),
                ..TextFace::default()
            },
        })
        .collect()
}
