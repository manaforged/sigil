use crate::app::cell_width;
use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{Dimension, cols};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::style::{NodeStyleExt, Px, StyleAtom, TextRun, TextRuns, TextStyle, Token};

use super::inline::{Styled, styled};

const RULE: StyleAtom = StyleAtom::BorderWidth(Px(1));

#[cfg(feature = "scene")]
fn shaped(text: &str, style: &TextStyle) -> Option<f32> {
    use crate::hal::ParagraphMetrics as _;
    let engine = crate::text::OwnedTextEngine::thread();
    let paragraph = engine.layout_paragraph_styled(text, None, style).ok()?;
    Some(paragraph.width())
}

#[cfg(not(feature = "scene"))]
const fn shaped(_: &str, _: &TextStyle) -> Option<f32> {
    None
}

const TOKEN_CAP: f32 = 30.0;
const TOKEN_FLOOR: f32 = 10.0;

#[derive(Clone, Copy)]
struct Extent {
    min: f32,
    max: f32,
}

#[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
fn measure(body: &str, runs: &[TextRun], bold: bool) -> f32 {
    let style = TextStyle {
        font_weight: if bold { 700 } else { 400 },
        runs: (!runs.is_empty()).then(|| TextRuns(runs.to_vec().into())),
        ..TextStyle::default()
    };
    let cells = cols(cell_width(body) as f32).px();
    let window = shaped(body, &style).unwrap_or(cells);
    CellForm::new(window.ceil(), cells).pick()
}

fn extent(body: &Styled, bold: bool) -> Extent {
    let lines = body.text.lines().map(|line| {
        if line.len() == body.text.len() {
            measure(line, &body.runs, bold)
        } else {
            measure(line, &[], bold)
        }
    });
    let max = lines.fold(0.0, f32::max);
    let mut min = 0.0_f32;
    let mut at = 0;
    for (index, _) in unicode_linebreak::linebreaks(&body.text) {
        let word = body.text.get(at..index).unwrap_or_default().trim_end();
        at = index;
        if !word.is_empty() {
            min = min.max(measure(word, &[], bold));
        }
    }
    Extent {
        min: min.min(cols(TOKEN_CAP).px()).min(max),
        max,
    }
}

fn extents(cells: &[Vec<Styled>]) -> Vec<Extent> {
    let columns = cells.iter().map(Vec::len).max().unwrap_or(0);
    let floor = cols(1.0).px();
    let mut out = vec![
        Extent {
            min: floor,
            max: floor,
        };
        columns
    ];
    for (index, row) in cells.iter().enumerate() {
        for (column, cell) in out.iter_mut().zip(row) {
            let next = extent(cell, index == 0);
            column.min = column.min.max(next.min);
            column.max = column.max.max(next.max);
        }
    }
    out
}

fn cell(body: Option<Styled>, extent: Extent, last: bool) -> NodeBuilder {
    let edge = cols(3.0).px();
    let basis = extent.min + edge;
    let mut node = container().role(Role::Cell).layout(
        LayoutStyle::block()
            .flex_basis(basis)
            .grow(extent.max - extent.min)
            .shrink((extent.min - cols(TOKEN_FLOOR).px()).max(0.0) / basis)
            .min_width(0.0)
            .max_width(extent.max + edge)
            .padding_x(cols(1.0)),
    );
    if !last {
        node = node.class([StyleAtom::BorderRight(Px(1)), Token::Border.border()]);
    }
    let body = body.unwrap_or_default();
    node.child(
        text(body.text)
            .runs(body.runs)
            .class(Token::OnSurface.text()),
    )
}

pub(super) fn table(rows: &[Vec<String>]) -> NodeBuilder {
    let cells: Vec<Vec<Styled>> = rows
        .iter()
        .map(|row| row.iter().map(|body| styled(body)).collect())
        .collect();
    let widths = extents(&cells);
    let total = widths
        .iter()
        .map(|width| width.max + cols(3.0).px())
        .sum::<f32>()
        + cols(2.0).px();
    let mut table = container()
        .role(Role::Table)
        .layout(
            LayoutStyle::flex_column()
                .width(total)
                .max_width(Dimension::Percent(1.0)),
        )
        .class([RULE, Token::Border.border(), Token::Field.bg()]);
    let count = cells.len();
    for (index, row) in cells.into_iter().enumerate() {
        let mut line = container()
            .role(Role::Row)
            .layout(LayoutStyle::flex_row().fill_width());
        if index == 0 && count > 1 {
            line = line.class([StyleAtom::BorderBottom(Px(1)), Token::Border.border()]);
        }
        if index == 0 {
            line = line.class("font-bold");
        }
        let mut row = row.into_iter();
        for (column, width) in widths.iter().enumerate() {
            line = line.child(cell(row.next(), *width, column + 1 == widths.len()));
        }
        table = table.child(line);
    }
    table
}
