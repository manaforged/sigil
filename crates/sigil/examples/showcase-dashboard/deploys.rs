use sigil::prelude::{
    AlignItems, ColWidth, FontFamily, Identifiable, LayoutBuilderExt, LayoutStyle, MenuItem,
    NodeBuilder, Paint, TableColumn, TableState, Token, Tone, chip, container, effect, table, text,
};
use sigil::reactive::untrack;
use sigil::style::space;
use sigil::widgets::{ChipBuilder, TableBuilder};

use super::data::{
    BRANCH_WIDTH, COMMIT_MIN_WIDTH, DURATION_WIDTH, Job, JobStatus, REGION_WIDTH, SHA_WIDTH,
    STATUS_WIDTH, Service, duration_label,
};
use super::live::Console;
use super::seed::JOBS;

#[derive(Clone, Copy, Debug)]
pub(crate) struct JobRow {
    pub(crate) slot: usize,
    pub(crate) job: &'static Job,
}

impl Identifiable for JobRow {
    type Id = usize;

    fn id(&self) -> usize {
        self.slot
    }
}

fn rows_for(service: Service) -> Vec<JobRow> {
    JOBS.iter()
        .enumerate()
        .filter(|(_, job)| job.service == service)
        .map(|(slot, job)| JobRow { slot, job })
        .collect()
}

pub(crate) fn job_state(console: Console) -> TableState<JobRow> {
    let state = TableState::identified(rows_for(console.service.peek()), columns(console));
    let sync = state.clone();
    effect(move || {
        let service = console.service.get();
        untrack(|| {
            sync.replace_rows(rows_for(service));
            sync.select_only(0);
        });
    });
    let selection = state.selection();
    let rows = state.rows();
    effect(move || {
        let first = selection.with(|picked| picked.iter().min().copied());
        let slot = first
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| rows.with(|all| all.get(index).map(|row| row.slot)));
        if let Some(slot) = slot {
            console.focus.set(slot);
        }
    });
    state
}

pub(crate) fn job_table(console: Console, state: TableState<JobRow>) -> TableBuilder<JobRow> {
    table(state)
        .selectable_checkboxes()
        .viewport_fill()
        .row_menu(move |row: &JobRow| {
            let slot = row.slot;
            vec![
                MenuItem::new("Retry").activate(move || console.retry(slot)),
                MenuItem::new("Roll back").activate(move || console.roll_back(slot)),
            ]
        })
}

fn contains_query(fields: &[&str], query: &str) -> bool {
    let query = query.trim().to_lowercase();
    fields
        .iter()
        .any(|field| field.to_lowercase().contains(&query))
}

fn columns(console: Console) -> Vec<TableColumn<JobRow>> {
    vec![
        TableColumn::new("commit", "Commit")
            .width(ColWidth::Fraction(1.0))
            .min_width(COMMIT_MIN_WIDTH)
            .filterable(|row: &JobRow, query: &str| {
                contains_query(&[row.job.sha, row.job.message, row.job.author], query)
            })
            .cell(|row: &JobRow| commit_cell(row.job)),
        TableColumn::new("branch", "Branch")
            .width(ColWidth::Fixed(BRANCH_WIDTH))
            .sort_by(|row: &JobRow| row.job.branch)
            .filterable(|row: &JobRow, query: &str| contains_query(&[row.job.branch], query))
            .cell(|row: &JobRow| plain(row.job.branch)),
        TableColumn::new("region", "Region")
            .width(ColWidth::Fixed(REGION_WIDTH))
            .sort_by(|row: &JobRow| row.job.region)
            .cell(|row: &JobRow| plain(row.job.region.label())),
        TableColumn::new("duration", "Duration")
            .numeric()
            .width(ColWidth::Fixed(DURATION_WIDTH))
            .sort_by(|row: &JobRow| row.job.secs)
            .cell(move |row: &JobRow| duration_cell(console, row.slot, row.job.secs)),
        TableColumn::new("status", "Status")
            .width(ColWidth::Fixed(STATUS_WIDTH))
            .sort_by(move |row: &JobRow| console.status_of(row.slot))
            .cell(move |row: &JobRow| status_chip(console, row.slot)),
    ]
}

fn plain(value: &'static str) -> sigil::kernel::TextBuilder {
    text(value).nowrap().ellipsize()
}

fn commit_cell(job: &'static Job) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .min_width(0.0)
                .gap(space(2))
                .align_items(AlignItems::Center),
        )
        .child(
            text(job.sha)
                .nowrap()
                .font_family(FontFamily::Mono)
                .text_color(Token::Accent11)
                .layout(LayoutStyle::block().width(SHA_WIDTH).shrink(0.0)),
        )
        .child(
            text(job.message)
                .nowrap()
                .ellipsize()
                .layout(LayoutStyle::block().grow(1.0).min_width(0.0)),
        )
}

fn duration_cell(console: Console, slot: usize, secs: u32) -> sigil::kernel::TextBuilder {
    text(move || {
        if console.status_of(slot) == JobStatus::Queued {
            "—".to_owned()
        } else {
            duration_label(secs)
        }
    })
    .nowrap()
    .tabular_nums(true)
}

pub(crate) fn status_chip(console: Console, slot: usize) -> ChipBuilder {
    chip(
        move || console.status_of(slot).label().to_owned(),
        Tone::Neutral,
    )
    .tone(move || console.status_of(slot).tone())
    .dot()
}
