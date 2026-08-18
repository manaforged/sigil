//! Sigil side of the 1:1 frame: construct the tree, then `compute`.
//!
//! Host setup (`Cx`, measurer) is outside the clock. The timed window
//! is `add` + `compute` (+ clipped emit when `layout_paint`). That is
//! the same work GPUI times as `uniform_list` / `div` build +
//! `layout_as_root` + `prepaint`.

use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use sigil::widgets::virtual_list_bind;
use sigil_hal::{LogicalSize, Point, Rect, Size};
use sigil_kernel::{Cx, NodeBuilder, NodeId, TextContent, container, text};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextEngineMeasure, TextMeasure};
use sigil_profile::{self as profile, Frame};
use sigil_reactive::{Runtime, Signal, signal};
use sigil_renderer::emit_scene_clipped;
use sigil_text::OwnedTextEngine;

use crate::{
    ACTION_CELL_W, EquivalenceCheck, FrameworkRunner, HostInfo, ID_CELL_W, KRAUSEST_ACTION,
    KRAUSEST_TEXT_CELLS, LABEL_CELL_W, ROW_WIDTH, SPACER_CELL_W, Surface, TABLE_CELL_W, Workload,
    krausest_id, krausest_label, visible_band,
};

/// Sigil framework runner.
#[derive(Debug, Default, Clone, Copy)]
pub struct SigilRunner;

impl SigilRunner {
    /// Construct a fresh runner.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl FrameworkRunner for SigilRunner {
    fn framework(&self) -> &'static str {
        "sigil"
    }

    fn run(&self, workload: &Workload) -> Result<crate::RunResult, String> {
        match workload.category.as_str() {
            "virtual-list" | "data-table" => run_virtual_list(workload),
            "flex-tree" => run_flex_tree(workload),
            "text-labels" => run_text_labels(workload),
            other => Err(format!(
                "SigilRunner does not implement workload category {other:?}"
            )),
        }
    }
}

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _: &TextContent, _: Size) -> LogicalSize {
        LogicalSize::new(40.0, 16.0)
    }
}

#[derive(Clone, Copy)]
struct RowData {
    #[allow(dead_code)]
    id: u32,
}

fn make_data(n: usize) -> Vec<RowData> {
    (0..n)
        .map(|i| RowData {
            id: u32::try_from(i).unwrap_or(u32::MAX),
        })
        .collect()
}

pub(super) fn new_engine(real_text: bool) -> LayoutEngine {
    if real_text {
        LayoutEngine::with_text_measure(Box::new(TextEngineMeasure::new(OwnedTextEngine::thread())))
    } else {
        LayoutEngine::with_text_measure(Box::new(StubMeasure))
    }
}

#[derive(Clone, Copy)]
pub(super) struct SceneCount {
    pub draws: u64,
    pub glyphs: u64,
}

pub(super) fn paint(cx: &Cx, root: NodeId, surface: Surface, vw: f32, vh: f32) -> SceneCount {
    if surface != Surface::LayoutPaint {
        return SceneCount {
            draws: 0,
            glyphs: 0,
        };
    }
    let clip = Some(Rect::new(Point::new(0.0, 0.0), vw, vh));
    let scene = emit_scene_clipped(cx, root, clip);
    let mut draws = 0_u64;
    let mut glyphs = 0_u64;
    for cmd in &scene.commands {
        match cmd {
            sigil_renderer::SceneCommand::GlyphRun(_) => {
                glyphs += 1;
                draws += 1;
            }
            sigil_renderer::SceneCommand::Rect(_) => draws += 1,
            _ => {}
        }
    }
    SceneCount { draws, glyphs }
}

fn krausest_row(row_h: f32) -> impl Fn(usize, &RowData) -> NodeBuilder {
    move |_index, data| {
        let cell = |w: f32| container().layout(LayoutStyle::block().fixed_size(w, row_h));
        container()
            .layout(LayoutStyle::flex_row().fixed_size(ROW_WIDTH, row_h))
            .child(cell(ID_CELL_W).child(text(krausest_id(data.id))))
            .child(cell(LABEL_CELL_W).child(text(krausest_label(data.id))))
            .child(cell(ACTION_CELL_W).child(text(KRAUSEST_ACTION)))
            .child(cell(SPACER_CELL_W))
    }
}

fn table_row(row_h: f32, cols: usize) -> impl Fn(usize, &RowData) -> NodeBuilder {
    move |index, _data| {
        #[allow(clippy::cast_precision_loss)]
        let width = TABLE_CELL_W * cols as f32;
        let mut row = container().layout(LayoutStyle::flex_row().fixed_size(width, row_h));
        for col in 0..cols {
            row = row.child(
                container()
                    .layout(LayoutStyle::block().fixed_size(TABLE_CELL_W, row_h))
                    .child(text(format!("{index}:{col}"))),
            );
        }
        row
    }
}

fn add_list(
    cx: &mut Cx,
    rows: Signal<Rc<Vec<RowData>>>,
    scroll: Signal<f32>,
    p: &crate::WorkloadParams,
    table: bool,
) -> NodeId {
    let row_h = p.row_height_px;
    let cols = p.cells();
    if table {
        cx.add(
            virtual_list_bind(rows, table_row(row_h, cols))
                .row_height(row_h)
                .viewport_height(p.viewport_height_px)
                .viewport_width(p.viewport_width_px)
                .scroll_bind(scroll)
                .keyboard(false),
        )
    } else {
        cx.add(
            virtual_list_bind(rows, krausest_row(row_h))
                .row_height(row_h)
                .viewport_height(p.viewport_height_px)
                .viewport_width(p.viewport_width_px)
                .scroll_bind(scroll)
                .keyboard(false),
        )
    }
}

fn run_virtual_list(workload: &Workload) -> Result<crate::RunResult, String> {
    if workload.params.gpui_table() {
        return Runtime::scope("sigil-bench-parity/quote-table", || {
            super::quote_table::run(workload)
        });
    }
    run_generic_list(workload)
}

fn run_generic_list(workload: &Workload) -> Result<crate::RunResult, String> {
    let p = &workload.params;
    validate(p.row_count, p.iterations)?;
    if p.reuse {
        return run_reuse_list(workload);
    }
    let table = workload.category == "data-table";
    let available = Size::fixed(p.viewport_width_px, p.viewport_height_px);
    let surface = p.surface();
    let data = Rc::new(make_data(p.row_count));
    let book = data.len();
    #[allow(clippy::cast_precision_loss)]
    let max_scroll = (p.row_count as f32).mul_add(p.row_height_px, -p.viewport_height_px);
    let mut y = if p.tail() { max_scroll.max(0.0) } else { 0.0 };
    profile::enable();
    profile::reset();
    let mut last_visible = 0usize;
    let mut last_first = 0usize;
    let mut step = || {
        if !p.tail() {
            y += p.row_height_px;
            if y > max_scroll {
                y = 0.0;
            }
        }
        // One runtime per sample so keyed-row effects die with the tree.
        Runtime::scope("sigil-bench-parity/virtual-list", || {
            let rows = signal(Rc::clone(&data));
            let scroll = signal(y);
            let mut cx = Cx::new_root();
            let mut engine = new_engine(true);
            profile::begin_frame();
            let start = Instant::now();
            let root = add_list(&mut cx, rows, scroll, p, table);
            let _ = engine.compute(&mut cx, root, available);
            let _cmds = paint(
                &cx,
                root,
                surface,
                p.viewport_width_px,
                p.viewport_height_px,
            );
            profile::tick_frame();
            let miss = u64::from(engine.generation() > 0);
            last_visible = visible_row_count(&cx, root);
            last_first = first_from_scroll(y, p.row_height_px);
            (
                u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX),
                miss,
            )
        })
    };
    let (samples_ns, misses) = drain_steps(p, &mut step);
    Ok(finish_list(
        workload,
        samples_ns,
        misses,
        last_visible,
        book,
        last_first,
    ))
}

fn run_reuse_list(workload: &Workload) -> Result<crate::RunResult, String> {
    let p = &workload.params;
    let table = workload.category == "data-table";
    let available = Size::fixed(p.viewport_width_px, p.viewport_height_px);
    let surface = p.surface();
    let data = Rc::new(make_data(p.row_count));
    let book = data.len();
    #[allow(clippy::cast_precision_loss)]
    let max_scroll = (p.row_count as f32).mul_add(p.row_height_px, -p.viewport_height_px);
    Runtime::scope("sigil-bench-parity/virtual-list-reuse", || {
        let rows = signal(Rc::clone(&data));
        let scroll = signal(if p.tail() { max_scroll.max(0.0) } else { 0.0 });
        let mut cx = Cx::new_root();
        let mut engine = new_engine(true);
        let root = add_list(&mut cx, rows, scroll, p, table);
        let _ = engine.compute(&mut cx, root, available);
        profile::enable();
        profile::reset();
        let mut step = || {
            if !p.tail() {
                scroll.update(|value| {
                    *value += p.row_height_px;
                    if *value > max_scroll {
                        *value = 0.0;
                    }
                });
            }
            profile::begin_frame();
            let start = Instant::now();
            let before = engine.generation();
            let _ = engine.compute(&mut cx, root, available);
            let _cmds = paint(
                &cx,
                root,
                surface,
                p.viewport_width_px,
                p.viewport_height_px,
            );
            profile::tick_frame();
            let miss = u64::from(engine.generation() > before);
            (
                u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX),
                miss,
            )
        };
        let (samples_ns, misses) = drain_steps(p, &mut step);
        Ok(finish_list(
            workload,
            samples_ns,
            misses,
            visible_row_count(&cx, root),
            book,
            first_from_scroll(scroll.get(), p.row_height_px),
        ))
    })
}

/// Verify + assemble the run result for a generic-list workload.
fn finish_list(
    workload: &Workload,
    samples_ns: Vec<u64>,
    misses: u64,
    visible: usize,
    book: usize,
    first: usize,
) -> crate::RunResult {
    let mut verify = verify_map(
        samples_ns.len(),
        misses,
        u64::try_from(visible).unwrap_or(0),
        last_measure_count(),
    );
    verify.insert("book_rows".into(), u64::try_from(book).unwrap_or(0));
    verify.insert("first_visible".into(), u64::try_from(first).unwrap_or(0));
    let mut extra = check_visible(visible, &workload.params);
    extra.insert(
        "book_held".into(),
        crate::book_held(book, workload.params.row_count),
    );
    extra.insert(
        "layout_did_work".into(),
        if crate::layout_void(&verify) {
            crate::EquivalenceCheck::Fail {
                detail: "layout_hits == samples".into(),
            }
        } else {
            crate::EquivalenceCheck::Pass
        },
    );
    extra.insert(
        "text_did_work".into(),
        krausest_text_ok(&verify, visible, workload.params.reuse),
    );
    if workload.params.tail() {
        extra.insert(
            "tail_window".into(),
            crate::tail_window(first, visible, workload.params.row_count),
        );
    }
    finish(workload, samples_ns, extra, verify)
}

fn krausest_text_ok(
    verify: &HashMap<String, u64>,
    visible: usize,
    reuse: bool,
) -> EquivalenceCheck {
    let got = verify.get("text_measure_last").copied().unwrap_or(0);
    let one = u64::try_from(KRAUSEST_TEXT_CELLS).unwrap_or(0);
    let band = u64::try_from(visible.saturating_mul(KRAUSEST_TEXT_CELLS)).unwrap_or(0);
    if visible == 0 || one == 0 {
        return EquivalenceCheck::Fail {
            detail: "no visible krausest text".into(),
        };
    }
    if reuse {
        return if got >= one {
            EquivalenceCheck::Pass
        } else {
            EquivalenceCheck::Fail {
                detail: format!("reuse text.measure {got} < {one} (new row)"),
            }
        };
    }
    if got >= band {
        EquivalenceCheck::Pass
    } else {
        EquivalenceCheck::Fail {
            detail: format!(
                "text.measure {got} < {band} (visible {visible} × {KRAUSEST_TEXT_CELLS})"
            ),
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn first_from_scroll(scroll: f32, row_h: f32) -> usize {
    if row_h <= 0.0 {
        0
    } else {
        (scroll / row_h).floor().max(0.0) as usize
    }
}

fn drain_steps(
    p: &crate::WorkloadParams,
    step: &mut impl FnMut() -> (u64, u64),
) -> (Vec<u64>, u64) {
    for _ in 0..p.warmup {
        let _ = step();
    }
    profile::reset();
    let mut samples_ns = Vec::with_capacity(p.iterations);
    let mut misses = 0_u64;
    for _ in 0..p.iterations {
        let (ns, miss) = step();
        samples_ns.push(ns);
        misses += miss;
    }
    (samples_ns, misses)
}

fn run_flex_tree(workload: &Workload) -> Result<crate::RunResult, String> {
    let p = &workload.params;
    validate(p.row_count, p.iterations)?;
    let cells = p.cells();
    Ok(Runtime::scope("sigil-bench-parity/flex-tree", || {
        sample_build(workload, false, || {
            let mut col = container().layout(
                LayoutStyle::flex_column().fixed_size(p.viewport_width_px, p.viewport_height_px),
            );
            for _ in 0..p.row_count {
                let mut row = container().layout(
                    LayoutStyle::flex_row().fixed_size(p.viewport_width_px, p.row_height_px),
                );
                for _ in 0..cells {
                    row =
                        row.child(container().layout(
                            LayoutStyle::block().fixed_size(p.row_height_px, p.row_height_px),
                        ));
                }
                col = col.child(row);
            }
            col
        })
    }))
}

fn run_text_labels(workload: &Workload) -> Result<crate::RunResult, String> {
    let p = &workload.params;
    validate(p.row_count, p.iterations)?;
    Ok(Runtime::scope("sigil-bench-parity/text-labels", || {
        sample_build(workload, true, || {
            let mut col = container().layout(
                LayoutStyle::flex_column().fixed_size(p.viewport_width_px, p.viewport_height_px),
            );
            for i in 0..p.row_count {
                col = col.child(text(format!("row {i} label payload")));
            }
            col
        })
    }))
}

fn sample_build(
    workload: &Workload,
    real_text: bool,
    mut build: impl FnMut() -> NodeBuilder,
) -> crate::RunResult {
    let p = &workload.params;
    let available = Size::fixed(p.viewport_width_px, p.viewport_height_px);
    let surface = p.surface();
    profile::enable();
    profile::reset();
    let mut step = || {
        let mut cx = Cx::new_root();
        let mut engine = new_engine(real_text);
        profile::begin_frame();
        let start = Instant::now();
        let root = cx.add(build());
        let _ = engine.compute(&mut cx, root, available);
        paint(
            &cx,
            root,
            surface,
            p.viewport_width_px,
            p.viewport_height_px,
        );
        let ns = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        profile::tick_frame();
        (ns, u64::from(engine.generation() > 0))
    };
    for _ in 0..p.warmup {
        let _ = step();
    }
    profile::reset();
    let mut samples_ns = Vec::with_capacity(p.iterations);
    let mut misses = 0_u64;
    let mut measures = 0_u64;
    for _ in 0..p.iterations {
        let (ns, miss) = step();
        samples_ns.push(ns);
        misses += miss;
        measures = measures.max(last_measure_count());
    }
    let verify = verify_map(samples_ns.len(), misses, 0, measures);
    let extra = build_checks(&verify, real_text, measures);
    finish(workload, samples_ns, extra, verify)
}

fn build_checks(
    verify: &HashMap<String, u64>,
    real_text: bool,
    measures: u64,
) -> HashMap<String, EquivalenceCheck> {
    let mut extra = HashMap::new();
    extra.insert(
        "layout_did_work".into(),
        if crate::layout_void(verify) {
            EquivalenceCheck::Fail {
                detail: "layout_hits == samples".into(),
            }
        } else {
            EquivalenceCheck::Pass
        },
    );
    if real_text {
        extra.insert(
            "text_did_work".into(),
            if measures > 0 {
                EquivalenceCheck::Pass
            } else {
                EquivalenceCheck::Fail {
                    detail: "text.measure == 0 on every sample".into(),
                }
            },
        );
    }
    extra
}

pub(super) fn validate(rows: usize, iterations: usize) -> Result<(), String> {
    if rows == 0 {
        return Err("row_count must be > 0".into());
    }
    if iterations == 0 {
        return Err("iterations must be > 0".into());
    }
    Ok(())
}

pub(super) fn last_measure_count() -> u64 {
    profile::frames()
        .last()
        .map_or(0, |frame: &Frame| frame.counter("text.measure"))
}

pub(super) fn verify_map(
    samples: usize,
    misses: u64,
    visible: u64,
    measures: u64,
) -> HashMap<String, u64> {
    let samples = u64::try_from(samples).unwrap_or(0);
    let hits = samples.saturating_sub(misses);
    let mut out = HashMap::new();
    out.insert("samples".into(), samples);
    out.insert("layout_hits".into(), hits);
    out.insert("layout_misses".into(), misses);
    out.insert("visible_last".into(), visible);
    out.insert("text_measure_last".into(), measures);
    out
}

pub(super) fn finish(
    workload: &Workload,
    samples_ns: Vec<u64>,
    extra: HashMap<String, EquivalenceCheck>,
    mut verify: HashMap<String, u64>,
) -> crate::RunResult {
    verify
        .entry("reuse".into())
        .or_insert(u64::from(workload.params.reuse));
    verify
        .entry("tick".into())
        .or_insert(u64::from(workload.params.tick()));
    let mut checks = extra;
    for spec in &workload.equivalence_checks {
        checks
            .entry(spec.name.clone())
            .or_insert(EquivalenceCheck::NotApplicable);
    }
    eprintln!(
        "VERIFY {} median_us={:.2} hits={} misses={} visible={} text.measure={}",
        workload.workload_id,
        crate::RunSummary::from_samples(&samples_ns).median_us,
        verify.get("layout_hits").copied().unwrap_or(0),
        verify.get("layout_misses").copied().unwrap_or(0),
        verify.get("visible_last").copied().unwrap_or(0),
        verify.get("text_measure_last").copied().unwrap_or(0),
    );
    crate::RunResult {
        framework: "sigil".into(),
        workload_id: workload.workload_id.clone(),
        summary: crate::RunSummary::from_samples(&samples_ns),
        samples_ns,
        equivalence_check_results: checks,
        host: HostInfo::capture(),
        verify,
    }
}

pub(super) fn check_visible(
    actual: usize,
    p: &crate::WorkloadParams,
) -> HashMap<String, EquivalenceCheck> {
    let (_, vh) = p.viewport();
    let (lo, hi) = visible_band(vh, p.row_h(), p.row_count);
    let mut out = HashMap::new();
    out.insert(
        "visible_window_indices".into(),
        if actual >= lo && actual <= hi {
            EquivalenceCheck::Pass
        } else {
            EquivalenceCheck::Fail {
                detail: format!("visible rows {actual} not in {lo}..={hi}"),
            }
        },
    );
    out
}

pub(super) fn visible_row_count(cx: &Cx, root: NodeId) -> usize {
    let Some(host) = cx.first_child(root) else {
        return 0;
    };
    let Some(content) = cx.first_child(host) else {
        return cx.child_count(host);
    };
    let Some(window) = cx.first_child(content) else {
        return cx.child_count(content);
    };
    cx.child_count(window)
}

#[cfg(test)]
mod tests {
    use super::SigilRunner;
    use crate::{
        EquivalenceCheck, EquivalenceCheckSpec, FrameworkRunner, KRAUSEST_TEXT_CELLS, Workload,
        WorkloadParams,
    };

    fn krausest_scroll() -> Workload {
        Workload {
            workload_id: "virtual-list-krausest-smoke-scroll".into(),
            description: String::new(),
            category: "virtual-list".into(),
            params: WorkloadParams {
                row_count: 200,
                row_height_px: 32.0,
                viewport_width_px: 1280.0,
                viewport_height_px: 1280.0,
                row_shape: "krausest-4-cell".into(),
                scroll_pattern: "linear-one-row-per-frame".into(),
                iterations: 4,
                warmup: 1,
                surface: "layout".into(),
                reuse: true,
                cells_per_row: 4,
            },
            equivalence_checks: vec![EquivalenceCheckSpec {
                name: "text_did_work".into(),
                description: String::new(),
            }],
        }
    }

    #[test]
    fn scroll_measures_new_row_text() {
        let result = SigilRunner::new().run(&krausest_scroll()).expect("runner");
        let got = result.verify.get("text_measure_last").copied().unwrap_or(0);
        assert!(
            got >= u64::try_from(KRAUSEST_TEXT_CELLS).unwrap_or(0),
            "verify={:?}",
            result.verify
        );
        assert!(
            matches!(
                result.equivalence_check_results.get("text_did_work"),
                Some(EquivalenceCheck::Pass)
            ),
            "text_did_work {:?}",
            result.equivalence_check_results
        );
    }
}
