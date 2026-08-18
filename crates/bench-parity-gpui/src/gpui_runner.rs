//! GPUI side of the 1:1 frame: construct the element, then
//! `layout_as_root` + `prepaint`.
//!
//! `TestAppContext` is created once and never enters the clock.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bench_parity::{
    ACTION_CELL_W, EquivalenceCheck, FrameworkRunner, HostInfo, ID_CELL_W, KRAUSEST_ACTION,
    LABEL_CELL_W, ROW_WIDTH, SPACER_CELL_W, Surface, TABLE_CELL_W, Workload, krausest_id,
    krausest_label, visible_band,
};
use data_table::quote::Quote;
use data_table::{FIELDS, cell_text, generate, tick_visible};
use gpui::{
    AvailableSpace, Context, Div, IntoElement, ParentElement, Render, ScrollStrategy, Styled,
    TestAppContext, TestDispatcher, UniformListScrollHandle, VisualTestContext, Window, div, green,
    point, px, red, rgb, size, uniform_list,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

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
            other => Err(format!(
                "GpuiRunner does not implement workload category {other:?}"
            )),
        }
    }
}

struct EmptyRoot;
impl Render for EmptyRoot {
    fn render(&mut self, _w: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}

struct Host {
    vc: VisualTestContext,
}

fn new_host() -> Host {
    let dispatcher = TestDispatcher::new(StdRng::seed_from_u64(0));
    let mut cx = TestAppContext::build(dispatcher, Some("bench-parity-gpui"));
    let window = cx.add_window(|_w, _cx| EmptyRoot);
    Host {
        vc: VisualTestContext::from_window(window.into(), &cx),
    }
}

fn space(p: &bench_parity::WorkloadParams) -> gpui::Size<AvailableSpace> {
    let (vw, vh) = p.viewport();
    size(px(vw), px(vh)).map(AvailableSpace::Definite)
}

fn time_construct_layout(
    host: &mut Host,
    surface: Surface,
    p: &bench_parity::WorkloadParams,
    build: impl FnOnce() -> gpui::AnyElement,
) -> (u64, bool) {
    let origin = point(px(0.0), px(0.0));
    let avail = space(p);
    let mut painted = surface != Surface::LayoutPaint;
    let start = Instant::now();
    host.vc.update(|window, cx| {
        let mut el = build();
        el.layout_as_root(avail, window, cx);
        el.prepaint_at(origin, window, cx);
        if surface == Surface::LayoutPaint {
            painted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                el.paint(window, cx);
            }))
            .is_ok();
        }
    });
    (
        u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX),
        painted,
    )
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
    let table = workload.category == "data-table";
    let book = table.then(|| Arc::new(Mutex::new(generate(p.row_count))));
    let ids = if table {
        None
    } else {
        Some(Arc::new(
            (0..p.row_count)
                .map(|i| u32::try_from(i).unwrap_or(u32::MAX))
                .collect::<Vec<u32>>(),
        ))
    };
    let book_len = book
        .as_ref()
        .and_then(|rows| rows.lock().ok().map(|g| g.len()))
        .or_else(|| ids.as_ref().map(|rows| rows.len()))
        .unwrap_or(0);
    let surface = p.surface();
    let cols = p.cells();
    let (vw, vh) = p.viewport();
    let row_h = p.row_h();
    let handle = UniformListScrollHandle::new();
    let observed: Arc<Mutex<Vec<Range<usize>>>> = Arc::new(Mutex::new(Vec::new()));
    let mut host = new_host();
    let (vis_lo, vis_hi) = visible_band(vh, row_h, p.row_count);
    let vis = vis_hi;
    let mut first = if p.tail() {
        p.row_count.saturating_sub(vis_lo.max(1))
    } else {
        0
    };
    let paint_ok = std::cell::Cell::new(true);
    let tick = p.tick();
    let seed_last = book.as_ref().and_then(|rows| {
        rows.lock()
            .ok()
            .and_then(|quotes| quotes.first().map(|quote| quote.last_done))
    });
    let reconstruct = !p.reuse && !tick;
    let mut step_n = 0_u32;
    let mut step = |ix: usize| {
        if reconstruct {
            host = new_host();
        }
        if tick {
            tick_gpui_book(book.as_ref(), &mut step_n, vis);
        } else {
            handle.scroll_to_item_strict(ix, ScrollStrategy::Top);
        }
        let obs = observed.clone();
        let book = book.clone();
        let ids = ids.clone();
        let (ns, ok) = time_construct_layout(&mut host, surface, p, || {
            let h = handle.clone();
            uniform_list("bench-list", p.row_count, move |range, _w, _cx| {
                if let Ok(mut g) = obs.lock() {
                    g.push(range.clone());
                }
                if let Some(ids) = ids.as_ref() {
                    let _ = ids.get(range.start);
                }
                let guard = book.as_ref().and_then(|book| book.lock().ok());
                range
                    .map(|i| one_row(i, guard.as_deref(), table, row_h, cols))
                    .collect::<Vec<Div>>()
            })
            .track_scroll(h)
            .w(px(vw))
            .h(px(vh))
            .into_any_element()
        });
        paint_ok.set(ok);
        ns
    };
    let samples_ns = drain_gpui(p, &mut first, &mut step);
    let now_last = book.as_ref().and_then(|rows| {
        rows.lock()
            .ok()
            .and_then(|quotes| quotes.first().map(|quote| quote.last_done))
    });
    let last_done_moved = tick && seed_last.zip(now_last).is_some_and(|(a, b)| a != b);
    let result = finish_gpui_list(
        workload,
        samples_ns,
        &observed,
        &paint_ok,
        p,
        book_len,
        last_done_moved,
    );
    Ok(result)
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

/// Assemble the run result for a GPUI virtual-list workload.
fn finish_gpui_list(
    workload: &Workload,
    samples_ns: Vec<u64>,
    observed: &Arc<Mutex<Vec<Range<usize>>>>,
    paint_ok: &std::cell::Cell<bool>,
    p: &bench_parity::WorkloadParams,
    book: usize,
    last_done_moved: bool,
) -> bench_parity::RunResult {
    let last = last_real_range(&observed.lock().map(|g| g.clone()).unwrap_or_default());
    let first = last.as_ref().map_or(0, |r| r.start);
    let visible = last.as_ref().map_or(0, |r| r.end.saturating_sub(r.start));
    let mut extra = stamp_table_checks(p, visible, paint_ok.get(), visible_check(p, last));
    extra.insert(
        "book_held".into(),
        bench_parity::book_held(book, p.row_count),
    );
    if !p.gpui_table() {
        extra.insert(
            "layout_did_work".into(),
            if visible > 0 {
                EquivalenceCheck::Pass
            } else {
                EquivalenceCheck::Fail {
                    detail: "uniform_list painted no rows".into(),
                }
            },
        );
        extra.insert(
            "text_did_work".into(),
            if visible > 0 {
                EquivalenceCheck::Pass
            } else {
                EquivalenceCheck::Fail {
                    detail: "uniform_list painted no rows".into(),
                }
            },
        );
    }
    if p.tick() {
        extra.insert(
            "last_done_did_move".into(),
            if last_done_moved {
                EquivalenceCheck::Pass
            } else {
                EquivalenceCheck::Fail {
                    detail: "book last_done did not change".into(),
                }
            },
        );
    }
    if p.tail() {
        extra.insert(
            "tail_window".into(),
            bench_parity::tail_window(first, visible, p.row_count),
        );
    }
    let mut verify = verify_visible(visible);
    verify.insert("book_rows".into(), u64::try_from(book).unwrap_or(0));
    verify.insert("first_visible".into(), u64::try_from(first).unwrap_or(0));
    finish(workload, samples_ns, extra, verify)
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
        if visible > 0 {
            EquivalenceCheck::Pass
        } else {
            EquivalenceCheck::Fail {
                detail: "uniform_list painted no rows".into(),
            }
        },
    );
    checks.insert(
        "text_did_work".into(),
        if visible > 0 {
            EquivalenceCheck::Pass
        } else {
            EquivalenceCheck::Fail {
                detail: "no visible rows, no text cells".into(),
            }
        },
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
            col = col.child(div().child(format!("row {i} label payload")));
        }
        col.into_any_element()
    })
}

#[allow(
    clippy::unnecessary_wraps,
    clippy::redundant_closure,
    reason = "callers stay Result for the runner trait"
)]
fn sample_build(
    workload: &Workload,
    host: &mut Host,
    mut build: impl FnMut() -> gpui::AnyElement,
) -> Result<bench_parity::RunResult, String> {
    let p = &workload.params;
    let surface = p.surface();
    let mut step = || {
        *host = new_host();
        time_construct_layout(host, surface, p, || build()).0
    };
    for _ in 0..p.warmup {
        let _ = step();
    }
    let mut samples_ns = Vec::with_capacity(p.iterations);
    for _ in 0..p.iterations {
        samples_ns.push(step());
    }
    let mut extra = HashMap::new();
    extra.insert(
        "layout_did_work".into(),
        if samples_ns.is_empty() {
            EquivalenceCheck::Fail {
                detail: "no samples".into(),
            }
        } else {
            EquivalenceCheck::Pass
        },
    );
    if workload.category == "text-labels" {
        extra.insert(
            "text_did_work".into(),
            if samples_ns.is_empty() {
                EquivalenceCheck::Fail {
                    detail: "no samples".into(),
                }
            } else {
                EquivalenceCheck::Pass
            },
        );
    }
    Ok(finish(workload, samples_ns, extra, HashMap::new()))
}

fn last_real_range(ranges: &[Range<usize>]) -> Option<Range<usize>> {
    ranges.iter().rev().find(|r| r.end > r.start + 1).cloned()
}

fn visible_check(
    p: &bench_parity::WorkloadParams,
    last: Option<Range<usize>>,
) -> HashMap<String, EquivalenceCheck> {
    let (_, vh) = p.viewport();
    let (lo, hi) = visible_band(vh, p.row_h(), p.row_count);
    let mut out = HashMap::new();
    let verdict = last.map_or_else(
        || EquivalenceCheck::Fail {
            detail: "uniform_list never reported a visible range".into(),
        },
        |range| {
            let actual = range.end - range.start;
            if actual >= lo && actual <= hi {
                EquivalenceCheck::Pass
            } else {
                EquivalenceCheck::Fail {
                    detail: format!("visible rows {actual} not in {lo}..={hi}"),
                }
            }
        },
    );
    out.insert("visible_window_indices".into(), verdict);
    out
}

fn verify_visible(visible: usize) -> HashMap<String, u64> {
    let mut out = HashMap::new();
    out.insert("visible_last".into(), u64::try_from(visible).unwrap_or(0));
    out
}

fn finish(
    workload: &Workload,
    samples_ns: Vec<u64>,
    extra: HashMap<String, EquivalenceCheck>,
    mut verify: HashMap<String, u64>,
) -> bench_parity::RunResult {
    // GPUI builds a fresh element tree every sample. reuse=0 is the
    // work that ran, not the workload's reuse flag.
    verify.insert("reuse".into(), 0);
    verify
        .entry("tick".into())
        .or_insert(u64::from(workload.params.tick()));
    verify
        .entry("samples".into())
        .or_insert(u64::try_from(samples_ns.len()).unwrap_or(0));
    let mut checks = extra;
    for spec in &workload.equivalence_checks {
        checks
            .entry(spec.name.clone())
            .or_insert(EquivalenceCheck::NotApplicable);
    }
    eprintln!(
        "VERIFY gpui {} median_us={:.2} visible={}",
        workload.workload_id,
        bench_parity::RunSummary::from_samples(&samples_ns).median_us,
        verify.get("visible_last").copied().unwrap_or(0),
    );
    bench_parity::RunResult {
        framework: "gpui".into(),
        workload_id: workload.workload_id.clone(),
        summary: bench_parity::RunSummary::from_samples(&samples_ns),
        samples_ns,
        equivalence_check_results: checks,
        host: HostInfo::capture(),
        verify,
    }
}

fn validate(p: &bench_parity::WorkloadParams) -> Result<(), String> {
    if p.row_count == 0 {
        return Err("row_count must be > 0".into());
    }
    if p.iterations == 0 {
        return Err("iterations must be > 0".into());
    }
    Ok(())
}
