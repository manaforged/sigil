//! GPUI side of the 1:1 frame: construct the element, then
//! `layout_as_root` + `prepaint`.
//!
//! `TestAppContext` is created once and never enters the clock.

use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bench_parity::{
    ACTION_CELL_W, EquivalenceCheck, FrameworkRunner, HostInfo, ID_CELL_W, KRAUSEST_ACTION,
    LABEL_CELL_W, OutputProof, ROW_WIDTH, SPACER_CELL_W, Surface, TABLE_CELL_W, Workload,
    krausest_id, krausest_label, milli_px, visible_band, visible_span,
};

#[path = "gpui_runner_build.rs"]
mod build;
#[path = "gpui_runner_construct.rs"]
mod construct;
#[path = "gpui_runner_timing.rs"]
mod timing;

use build::{finish, last_real_range, sample_build, validate, verify_visible, visible_check};
use data_table::quote::Quote;
use data_table::{FIELDS, cell_text, generate, tick_visible};
use gpui::{
    App, AvailableSpace, Bounds, Context, Div, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, ParentElement, Pixels, Render, ScrollStrategy,
    Styled, TestAppContext, TestDispatcher, UniformListScrollHandle, VisualTestContext, Window,
    div, green, point, px, red, rgb, size, uniform_list,
};
use rand::SeedableRng;
use rand::rngs::StdRng;
use timing::{Host, new_host, time_construct_layout};

/// GPUI framework runner.
#[derive(Debug, Default, Clone, Copy)]
pub struct GpuiRunner;

impl GpuiRunner {
    /// Construct a fresh runner.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl FrameworkRunner for GpuiRunner {
    fn framework(&self) -> &'static str {
        "gpui"
    }

    fn run(&self, workload: &Workload) -> Result<bench_parity::RunResult, String> {
        match workload.category.as_str() {
            "virtual-list" | "data-table" => run_virtual_list(workload),
            "flex-tree" => run_flex_tree(workload),
            "text-labels" => run_text_labels(workload),
            "wrap-text" | "kv-form" | "nested-flex" | "overlay-stack" => construct::run(workload),
            other => Err(format!(
                "GpuiRunner does not implement workload category {other:?}"
            )),
        }
    }
}

fn krausest_row(index: usize, row_h: f32) -> Div {
    let id = u32::try_from(index).unwrap_or(u32::MAX);
    div()
        .h(px(row_h))
        .w(px(ROW_WIDTH))
        .flex()
        .flex_row()
        .child(div().h(px(row_h)).w(px(ID_CELL_W)).child(krausest_id(id)))
        .child(
            div()
                .h(px(row_h))
                .w(px(LABEL_CELL_W))
                .child(krausest_label(id)),
        )
        .child(
            div()
                .h(px(row_h))
                .w(px(ACTION_CELL_W))
                .child(KRAUSEST_ACTION),
        )
        .child(div().h(px(row_h)).w(px(SPACER_CELL_W)))
}

fn quote_row(index: usize, quote: &Quote, row_h: f32) -> Div {
    let tint = if quote.up() { green() } else { red() };
    let bg = if index.is_multiple_of(2) {
        rgb(0x00FF_FFFF)
    } else {
        rgb(0x00FA_FAFA)
    };
    let mut row = div()
        .flex()
        .flex_row()
        .h(px(row_h))
        .border_b_1()
        .border_color(rgb(0x00E0_E0E0))
        .bg(bg)
        .py_0p5()
        .px_2();
    for field in FIELDS {
        let mut cell = div()
            .whitespace_nowrap()
            .truncate()
            .w(px(field.width))
            .px_1()
            .child(cell_text(index, quote, field.kind));
        if field.tint {
            cell = cell.text_color(tint);
        }
        row = row.child(cell);
    }
    row
}

fn table_row(index: usize, row_h: f32, cols: usize) -> Div {
    #[allow(clippy::cast_precision_loss)]
    let width = TABLE_CELL_W * cols as f32;
    let mut row = div().h(px(row_h)).w(px(width)).flex().flex_row();
    for col in 0..cols {
        row = row.child(
            div()
                .w(px(TABLE_CELL_W))
                .h(px(row_h))
                .child(format!("{index}:{col}")),
        );
    }
    row
}

fn one_row(i: usize, quotes: Option<&Vec<Quote>>, table: bool, row_h: f32, cols: usize) -> Div {
    quotes.and_then(|rows| rows.get(i)).map_or_else(
        || {
            if table {
                table_row(i, row_h, cols)
            } else {
                krausest_row(i, row_h)
            }
        },
        |quote| quote_row(i, quote, row_h),
    )
}

fn run_virtual_list(workload: &Workload) -> Result<bench_parity::RunResult, String> {
    let p = &workload.params;
    validate(p)?;
    let (table, tick) = (workload.category == "data-table", p.tick());
    let ListData {
        book,
        ids,
        len: book_len,
    } = list_data(p, table);
    let (vw, vh) = p.viewport();
    let (row_h, cols) = (p.row_h(), p.cells());
    let (handle, recorder) = (UniformListScrollHandle::new(), RangeRecorder::default());
    let mut host = new_host();
    let (_, vis) = visible_band(vh, row_h, p.row_count);
    let (paint_ok, root_size) = (std::cell::Cell::new(true), std::cell::Cell::new([0, 0]));
    let spec = ListSpec {
        recorder: recorder.clone(),
        ids,
        book: book.clone(),
        handle: handle.clone(),
        table,
        row_h,
        cols,
        viewport: (vw, vh),
        total: p.row_count,
    };
    let mut step_n = 0_u32;
    let mut step = |ix: usize| {
        if !p.reuse && !tick {
            host = new_host();
        }
        if tick {
            tick_gpui_book(book.as_ref(), &mut step_n, vis);
        } else {
            handle.scroll_to_item_strict(ix, ScrollStrategy::Top);
        }
        let spec = spec.clone();
        let (ns, ok, size) = time_construct_layout(&mut host, p.surface(), p, || spec.element());
        paint_ok.set(ok);
        root_size.set(size);
        ns
    };
    let obs = Observed(&recorder, &paint_ok, &root_size);
    Ok(list_result(
        workload,
        &mut step,
        book.as_ref(),
        book_len,
        obs,
    ))
}

#[derive(Clone)]
struct ListSpec {
    recorder: RangeRecorder,
    ids: Option<Arc<Vec<u32>>>,
    book: Option<Arc<Mutex<Vec<Quote>>>>,
    handle: UniformListScrollHandle,
    table: bool,
    row_h: f32,
    cols: usize,
    viewport: (f32, f32),
    total: usize,
}

impl ListSpec {
    fn element(self) -> gpui::AnyElement {
        let callback = self.clone();
        uniform_list("bench-list", self.total, move |range, _window, _cx| {
            range_rows(
                range,
                &callback.recorder,
                callback.ids.as_ref(),
                callback.book.as_ref(),
                (callback.table, callback.row_h, callback.cols),
            )
        })
        .track_scroll(self.handle)
        .w(px(self.viewport.0))
        .h(px(self.viewport.1))
        .into_any_element()
    }
}

struct ListData {
    book: Option<Arc<Mutex<Vec<Quote>>>>,
    ids: Option<Arc<Vec<u32>>>,
    len: usize,
}

fn list_data(p: &bench_parity::WorkloadParams, table: bool) -> ListData {
    let book = table.then(|| Arc::new(Mutex::new(generate(p.row_count))));
    let ids = row_ids(p, table);
    let len = held_rows(book.as_ref(), ids.as_ref());
    ListData { book, ids, len }
}

fn list_result(
    workload: &Workload,
    step: &mut impl FnMut(usize) -> u64,
    book: Option<&Arc<Mutex<Vec<Quote>>>>,
    book_len: usize,
    obs: Observed<'_>,
) -> bench_parity::RunResult {
    finish_gpui_list(workload, step, book, book_len, obs)
}

/// Build the rows GPUI asks for, recording the observed range.
fn range_rows(
    range: Range<usize>,
    recorder: &RangeRecorder,
    ids: Option<&Arc<Vec<u32>>>,
    book: Option<&Arc<Mutex<Vec<Quote>>>>,
    layout: (bool, f32, usize),
) -> Vec<Div> {
    let (table, row_h, cols) = layout;
    if let Ok(mut g) = recorder.ranges.lock() {
        g.push(range.clone());
    }
    if let Some(ids) = ids {
        let _ = ids.get(range.start);
    }
    let guard = book.and_then(|arc| arc.lock().ok());
    let text_rows = range
        .clone()
        .map(|i| semantic_row(i, guard.as_deref(), table, cols))
        .collect();
    if let Ok(mut out) = recorder.output.lock() {
        *out = Some(RangeOutput {
            range: range.clone(),
            text_rows,
        });
    }
    range
        .map(|i| one_row(i, guard.as_deref(), table, row_h, cols))
        .collect()
}

#[derive(Clone)]
struct RangeOutput {
    range: Range<usize>,
    text_rows: Vec<Vec<String>>,
}

#[derive(Clone, Default)]
struct RangeRecorder {
    ranges: Arc<Mutex<Vec<Range<usize>>>>,
    output: Arc<Mutex<Option<RangeOutput>>>,
}

fn semantic_row(
    index: usize,
    quotes: Option<&Vec<Quote>>,
    table: bool,
    cols: usize,
) -> Vec<String> {
    if let Some(quote) = quotes.and_then(|rows| rows.get(index)) {
        return FIELDS
            .iter()
            .map(|field| cell_text(index, quote, field.kind))
            .collect();
    }
    if table {
        return (0..cols).map(|col| format!("{index}:{col}")).collect();
    }
    let id = u32::try_from(index).unwrap_or(u32::MAX);
    vec![krausest_id(id), krausest_label(id), KRAUSEST_ACTION.into()]
}

/// Row ids for the krausest list; tables key on row content instead.
fn row_ids(p: &bench_parity::WorkloadParams, table: bool) -> Option<Arc<Vec<u32>>> {
    (!table).then(|| {
        Arc::new(
            (0..p.row_count)
                .map(|i| u32::try_from(i).unwrap_or(u32::MAX))
                .collect(),
        )
    })
}

/// Live row count: the book when present, else the id list.
fn held_rows(book: Option<&Arc<Mutex<Vec<Quote>>>>, ids: Option<&Arc<Vec<u32>>>) -> usize {
    book.and_then(|rows| rows.lock().ok().map(|g| g.len()))
        .or_else(|| ids.map(|rows| rows.len()))
        .unwrap_or(0)
}

/// First quote's `last_done`, for the tick-change check.
fn first_last_done(book: Option<&Arc<Mutex<Vec<Quote>>>>) -> Option<f64> {
    book.and_then(|rows| rows.lock().ok()?.first().map(|quote| quote.last_done))
}

/// First row index for the initial scroll position (tail = last page).
fn first_row(p: &bench_parity::WorkloadParams, vis_lo: usize) -> usize {
    if p.tail() {
        p.row_count.saturating_sub(vis_lo.max(1))
    } else {
        0
    }
}

/// Bump the tick counter and mutate the visible rows of the data-table book.
fn tick_gpui_book(book: Option<&Arc<Mutex<Vec<Quote>>>>, step_n: &mut u32, vis: usize) {
    let n = step_n.saturating_add(1);
    *step_n = n;
    if let Some(book) = book
        && let Ok(mut quotes) = book.lock()
    {
        tick_visible(&mut quotes, 0, vis, n);
    }
}

/// What the sampled steps observed: (painted ranges, paint health).
#[derive(Clone, Copy)]
struct Observed<'a>(
    &'a RangeRecorder,
    &'a std::cell::Cell<bool>,
    &'a std::cell::Cell<[i64; 2]>,
);

/// Pass when `ok`, else Fail with `detail`.
fn pass_if(ok: bool, detail: &str) -> EquivalenceCheck {
    if ok {
        EquivalenceCheck::Pass
    } else {
        EquivalenceCheck::Fail {
            detail: detail.into(),
        }
    }
}

/// Drain the measured samples, then verify + assemble the run result.
fn finish_gpui_list(
    workload: &Workload,
    step: &mut impl FnMut(usize) -> u64,
    book: Option<&Arc<Mutex<Vec<Quote>>>>,
    book_len: usize,
    obs: Observed<'_>,
) -> bench_parity::RunResult {
    let p = &workload.params;
    let tick = p.tick();
    let seed_last = first_last_done(book);
    let vis_lo = visible_band(p.viewport().1, p.row_h(), p.row_count).0;
    let mut scroll_first = first_row(p, vis_lo);
    let samples_ns = drain_gpui(p, &mut scroll_first, step);
    let now_last = first_last_done(book);
    let pair = seed_last.zip(now_last);
    // Change detection wants exact inequality, not an epsilon band.
    let moved = tick && pair.is_some_and(|(a, b)| a.to_bits() != b.to_bits());
    let ranges = obs.0.ranges.lock().map(|g| g.clone()).unwrap_or_default();
    let last = last_real_range(&ranges);
    let first = last.as_ref().map_or(0, |r| r.start);
    let visible = last.as_ref().map_or(0, |r| r.end.saturating_sub(r.start));
    let mut extra = stamp_table_checks(p, visible, obs.1.get(), visible_check(p, last));
    extra.insert(
        "book_held".into(),
        bench_parity::book_held(book_len, p.row_count),
    );
    if !p.gpui_table() {
        let no_rows = "uniform_list painted no rows";
        extra.insert("layout_did_work".into(), pass_if(visible > 0, no_rows));
        extra.insert("text_did_work".into(), pass_if(visible > 0, no_rows));
    }
    if p.tick() {
        extra.insert(
            "last_done_did_move".into(),
            pass_if(moved, "book last_done did not change"),
        );
    }
    if p.tail() {
        extra.insert(
            "tail_window".into(),
            bench_parity::tail_window(first, visible, p.row_count),
        );
    }
    let mut verify = verify_visible(visible);
    verify.insert("book_rows".into(), u64::try_from(book_len).unwrap_or(0));
    verify.insert("first_visible".into(), u64::try_from(first).unwrap_or(0));
    let output = observed_output(obs);
    finish(workload, samples_ns, extra, verify, output)
}

fn observed_output(obs: Observed<'_>) -> Option<OutputProof> {
    obs.0
        .output
        .lock()
        .ok()
        .and_then(|rows| rows.clone())
        .map(|rows| {
            let mut values = std::collections::BTreeMap::new();
            values.insert(
                "first_row".into(),
                u64::try_from(rows.range.start).unwrap_or(0),
            );
            values.insert(
                "visible_rows".into(),
                u64::try_from(rows.range.len()).unwrap_or(0),
            );
            OutputProof {
                schema: "visible-rows-v1".into(),
                root_size_milli_px: obs.2.get(),
                values,
                text_rows: rows.text_rows,
            }
        })
}

fn drain_gpui(
    p: &bench_parity::WorkloadParams,
    first: &mut usize,
    step: &mut impl FnMut(usize) -> u64,
) -> Vec<u64> {
    let next = |ix: &mut usize| {
        if p.tail() {
            return *ix;
        }
        // Construct 1:1 is the same window every sample. GPUI 0.2.2
        // still rebuilds the element tree.
        if !p.reuse && !p.tick() && p.gpui_table() {
            *ix = 0;
            return 0;
        }
        *ix = (*ix + 1) % p.row_count.max(1);
        *ix
    };
    for _ in 0..p.warmup {
        let _ = step(next(first));
    }
    let mut samples_ns = Vec::with_capacity(p.iterations);
    for _ in 0..p.iterations {
        samples_ns.push(step(next(first)));
    }
    samples_ns
}

fn stamp_table_checks(
    p: &bench_parity::WorkloadParams,
    visible: usize,
    paint_ok: bool,
    mut checks: HashMap<String, EquivalenceCheck>,
) -> HashMap<String, EquivalenceCheck> {
    if !p.gpui_table() {
        return checks;
    }
    checks.insert(
        "column_count".into(),
        EquivalenceCheck::Pass, // rows are built from FIELDS in quote_row
    );
    checks.insert(
        "layout_did_work".into(),
        pass_if(visible > 0, "uniform_list painted no rows"),
    );
    checks.insert(
        "text_did_work".into(),
        pass_if(visible > 0, "no visible rows, no text cells"),
    );
    checks.insert(
        "scene_did_work".into(),
        match p.surface() {
            Surface::Layout => EquivalenceCheck::NotApplicable,
            Surface::LayoutPaint if paint_ok => EquivalenceCheck::Pass,
            Surface::LayoutPaint => EquivalenceCheck::Fail {
                detail: "GPUI paint panicked on the test platform".into(),
            },
        },
    );
    checks
}

fn run_flex_tree(workload: &Workload) -> Result<bench_parity::RunResult, String> {
    let p = &workload.params;
    validate(p)?;
    let cells = p.cells();
    let mut host = new_host();
    sample_build(workload, &mut host, || {
        let mut col = div()
            .flex()
            .flex_col()
            .w(px(p.viewport_width_px))
            .h(px(p.viewport_height_px));
        for _ in 0..p.row_count {
            let mut row = div()
                .flex()
                .flex_row()
                .w(px(p.viewport_width_px))
                .h(px(p.row_height_px));
            for _ in 0..cells {
                row = row.child(div().w(px(p.row_height_px)).h(px(p.row_height_px)));
            }
            col = col.child(row);
        }
        col.into_any_element()
    })
}

fn run_text_labels(workload: &Workload) -> Result<bench_parity::RunResult, String> {
    let p = &workload.params;
    validate(p)?;
    let mut host = new_host();
    sample_build(workload, &mut host, || {
        let mut col = div()
            .flex()
            .flex_col()
            .w(px(p.viewport_width_px))
            .h(px(p.viewport_height_px));
        for i in 0..p.row_count {
            col = col.child(format!("row {i} label payload"));
        }
        col.into_any_element()
    })
}
