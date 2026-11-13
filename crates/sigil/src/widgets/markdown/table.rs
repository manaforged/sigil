use crate::layout::Space;
use std::rc::Rc;

use crate::app::cell_width;
use crate::kernel::{NodeBuilder, Role, SortDirection, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{JustifyContent, LayoutStyle, Length, cols, px_or_zero};
use crate::reactive::{Signal, memo, signal};
use crate::style::{StyleAtom, TextRun, TextRuns, TextStyle, Token};
use crate::widgets::Widget;
use crate::widgets::button::button;

use super::inline::{StyledText, styled};
use super::table_data::{Align, Table};

type Sort = Signal<Option<(usize, SortDirection)>>;

const RULE: StyleAtom = StyleAtom::BorderWidth(1);

#[cfg(feature = "scene")]
fn shaped(text: &str, style: &TextStyle) -> Option<f32> {
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
    let cells = px_or_zero(cols(cell_width(body) as f32));
    let window = shaped(body, &style).unwrap_or(cells);
    CellForm::new(window.ceil(), cells).pick()
}

fn extent(body: &StyledText, bold: bool) -> Extent {
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
        min: min.min(px_or_zero(cols(TOKEN_CAP))).min(max),
        max,
    }
}

fn extents(cells: &[Vec<StyledText>]) -> Vec<Extent> {
    let columns = cells.iter().map(Vec::len).max().unwrap_or(0);
    let floor = px_or_zero(cols(1.0));
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

struct Grid {
    data: Table,
    cells: Vec<Vec<StyledText>>,
    widths: Vec<Extent>,
    sort: Sort,
}

impl Grid {
    fn cell(&self, row: usize, column: usize) -> NodeBuilder {
        let extent = self
            .widths
            .get(column)
            .copied()
            .unwrap_or(Extent { min: 0.0, max: 0.0 });
        let edge = px_or_zero(cols(3.0));
        let basis = extent.min + edge;
        let body = self
            .cells
            .get(row)
            .and_then(|cells| cells.get(column))
            .cloned()
            .unwrap_or_default();
        let justify = match self.data.align_at(column) {
            Align::Start => JustifyContent::Start,
            Align::Center => JustifyContent::Center,
            Align::End => JustifyContent::End,
        };
        let mut node = container().role(Role::Cell).layout(
            LayoutStyle::flex_row()
                .justify_content(justify)
                .flex_basis(basis)
                .grow(extent.max - extent.min)
                .shrink((extent.min - px_or_zero(cols(TOKEN_FLOOR))).max(0.0) / basis)
                .min_width(0.0)
                .max_width(extent.max + edge)
                .padding_x(cols(1.0)),
        );
        if column + 1 < self.widths.len() {
            node = node.class([StyleAtom::BorderRight(1), Token::Border.border()]);
        }
        node.child(
            text(body.text)
                .runs(body.runs)
                .class(Token::OnSurface.text())
                .layout(LayoutStyle::block().shrink(1.0).min_width(0.0)),
        )
    }

    fn row(&self, row: usize) -> NodeBuilder {
        let mut line = container()
            .role(Role::Row)
            .layout(LayoutStyle::flex_row().fill_width());
        for column in 0..self.widths.len() {
            line = line.child(self.cell(row, column));
        }
        line
    }

    fn header_cell(&self, column: usize) -> NodeBuilder {
        let sort = self.sort;
        let current = memo(move || {
            sort.get()
                .filter(|(at, _)| *at == column)
                .map(|(_, way)| way)
        });
        let arrow = container().children_reactive(move || {
            let mark = match current.get() {
                Some(SortDirection::Ascending) => " ▲",
                Some(SortDirection::Descending) => " ▼",
                None => "",
            };
            (!mark.is_empty()).then(|| text(mark).nowrap())
        });
        self.cell(0, column)
            .role(Role::ColumnHeader)
            .child(arrow)
            .sort_direction(current)
            .activate(move || sort.set(next_sort(sort.peek(), column)))
    }
}

const fn next_sort(
    current: Option<(usize, SortDirection)>,
    column: usize,
) -> Option<(usize, SortDirection)> {
    match current {
        Some((at, SortDirection::Ascending)) if at == column => {
            Some((column, SortDirection::Descending))
        }
        Some((at, SortDirection::Descending)) if at == column => None,
        _ => Some((column, SortDirection::Ascending)),
    }
}

pub(super) fn table(data: Table) -> NodeBuilder {
    let mut cells: Vec<Vec<StyledText>> = data
        .rows
        .iter()
        .map(|row| row.iter().map(|body| styled(body)).collect())
        .collect();
    let mut measured = cells.clone();
    if let Some(header) = measured.first_mut() {
        for cell in header {
            cell.text.push_str(" ▲");
        }
    }
    let widths = extents(&measured);
    cells.resize_with(data.rows.len(), Vec::new);
    let total = widths
        .iter()
        .map(|width| width.max + px_or_zero(cols(3.0)))
        .sum::<f32>()
        + px_or_zero(cols(2.0));
    let sortable = data.rows.len() > 2;
    let grid = Rc::new(Grid {
        data,
        cells,
        widths,
        sort: signal(None),
    });
    let table = container()
        .role(Role::Table)
        .layout(LayoutStyle::flex_column().fill_width())
        .class([RULE, Token::Border.border(), Token::Field.background()])
        .child(header(&grid, sortable))
        .child(body(&grid));
    let mut frame = container().layout(
        LayoutStyle::flex_column()
            .width(total)
            .max_width(Length::Percent(1.0))
            .gap(Space::Xs),
    );
    if CellForm::new(true, false).pick() && grid.data.rows.len() > 1 {
        frame = frame.child(actions(&grid));
    }
    frame.child(table)
}

fn header(grid: &Rc<Grid>, sortable: bool) -> NodeBuilder {
    let line = container()
        .role(Role::Row)
        .layout(LayoutStyle::flex_row().fill_width())
        .class("font-bold");
    let line = if grid.data.rows.len() > 1 {
        line.class([StyleAtom::BorderBottom(1), Token::Border.border()])
    } else {
        line
    };
    if !sortable {
        return (0..grid.widths.len()).fold(line, |line, column| line.child(grid.cell(0, column)));
    }
    (0..grid.widths.len()).fold(line, |line, column| line.child(grid.header_cell(column)))
}

fn body(grid: &Rc<Grid>) -> NodeBuilder {
    let grid = Rc::clone(grid);
    container()
        .layout(LayoutStyle::flex_column().fill_width())
        .children_reactive(move || {
            grid.data
                .order(grid.sort.get())
                .into_iter()
                .map(|row| grid.row(row))
                .collect::<Vec<_>>()
        })
}

fn actions(grid: &Rc<Grid>) -> NodeBuilder {
    let markdown = Rc::clone(grid);
    let csv = Rc::clone(grid);
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .justify_content(JustifyContent::End)
                .gap(Space::Xs),
        )
        .child(button("Copy Markdown").ghost().activate(move || {
            let order = markdown.data.order(markdown.sort.peek());
            copy(&markdown.data.markdown(&order));
        }))
        .child(button("Copy CSV").ghost().activate(move || {
            let order = csv.data.order(csv.sort.peek());
            copy(&csv.data.csv(&order));
        }))
}

fn copy(body: &str) {
    if let Err(err) = crate::clipboard::copy(body) {
        tracing::warn!(target: "sigil::markdown", error = %err, "table copy failed");
    }
}
