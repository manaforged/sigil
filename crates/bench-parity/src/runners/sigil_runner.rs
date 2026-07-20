//! Sigil framework runner.
//!
//! Layer-1 measurement: per-iteration widget tree construction into a
//! fresh `Cx` + one `LayoutEngine::compute` pass. No GPU, no paint —
//! the cross-framework parity surface is layout only until a Layer-2
//! paint workload lands.

use std::collections::HashMap;
use std::time::Instant;

use sigil::widgets::virtual_list;
use sigil_hal::{LogicalSize, Size};
use sigil_kernel::{Cx, NodeBuilder, NodeId, TextContent, container};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
use sigil_reactive::{Runtime, Signal, signal};

use crate::{EquivalenceCheck, FrameworkRunner, HostInfo, RunResult, RunSummary, Workload};

const ID_CELL_W: f32 = 80.0;
const LABEL_CELL_W: f32 = 320.0;
const ACTION_CELL_W: f32 = 80.0;
const SPACER_CELL_W: f32 = 800.0;
const ROW_WIDTH: f32 = ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W;

/// Sigil framework runner. Stateless.
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

    fn run(&self, workload: &Workload) -> Result<RunResult, String> {
        match workload.category.as_str() {
            "virtual-list" => run_virtual_list(workload),
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
    // Field is part of the row payload — kept for future per-row work
    // (text, conditional styling) the bench currently elides.
    #[allow(dead_code)]
    id: u32,
}

#[allow(clippy::cast_possible_truncation)]
fn make_data(row_count: usize) -> Vec<RowData> {
    (0..row_count).map(|i| RowData { id: i as u32 }).collect()
}

// `virtual_list` requires `Fn(usize, &T) -> NodeBuilder`; the `&RowData`
// reference is dictated by the widget contract, not a perf choice.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn krausest_row(_index: usize, _data: &RowData) -> NodeBuilder {
    let cell = |w: f32, h: f32| container().layout(LayoutStyle::block().fixed_size(w, h));
    let row_h = 32.0;
    container()
        .layout(LayoutStyle::flex_row().fixed_size(ROW_WIDTH, row_h))
        .child(cell(ID_CELL_W, row_h))
        .child(cell(LABEL_CELL_W, row_h))
        .child(cell(ACTION_CELL_W, row_h))
        .child(cell(SPACER_CELL_W, row_h))
}

#[derive(Clone, Copy)]
struct BuildParams {
    row_height: f32,
    viewport_width: f32,
    viewport_height: f32,
}

fn build_widget(cx: &mut Cx, data: Vec<RowData>, scroll_y: Signal<f32>, p: BuildParams) -> NodeId {
    cx.add(
        virtual_list(data, krausest_row)
            .row_height(p.row_height)
            .viewport_height(p.viewport_height)
            .viewport_width(p.viewport_width)
            .scroll_bind(scroll_y),
    )
}

fn run_virtual_list(workload: &Workload) -> Result<RunResult, String> {
    let params = &workload.params;
    if params.row_count == 0 {
        return Err("row_count must be > 0".into());
    }
    if params.iterations == 0 {
        return Err("iterations must be > 0".into());
    }
    let build = BuildParams {
        row_height: params.row_height_px,
        viewport_width: params.viewport_width_px,
        viewport_height: params.viewport_height_px,
    };
    let available = Size::fixed(build.viewport_width, build.viewport_height);
    let scroll_step = build.row_height;

    Runtime::scope("sigil-bench-parity/virtual-list", || {
        let data = make_data(params.row_count);
        let scroll_y = signal(0.0_f32);
        #[allow(clippy::cast_precision_loss)]
        let max_scroll =
            (params.row_count as f32).mul_add(build.row_height, -build.viewport_height);

        for _ in 0..params.warmup {
            advance_scroll(scroll_y, scroll_step, max_scroll);
            measure_one(data.clone(), scroll_y, build, available);
        }

        let mut samples_ns: Vec<u64> = Vec::with_capacity(params.iterations);
        for _ in 0..params.iterations {
            advance_scroll(scroll_y, scroll_step, max_scroll);
            let elapsed = measure_one(data.clone(), scroll_y, build, available);
            samples_ns.push(elapsed);
        }

        let equivalence_check_results = run_equivalence_checks(workload, &data, build);

        Ok(RunResult {
            framework: "sigil".into(),
            workload_id: workload.workload_id.clone(),
            summary: RunSummary::from_samples(&samples_ns),
            samples_ns,
            equivalence_check_results,
            host: HostInfo::capture(),
        })
    })
}

fn advance_scroll(scroll_y: Signal<f32>, step: f32, max_scroll: f32) {
    scroll_y.update(|value| {
        *value += step;
        if *value > max_scroll {
            *value = 0.0;
        }
    });
}

fn measure_one(data: Vec<RowData>, scroll_y: Signal<f32>, p: BuildParams, available: Size) -> u64 {
    let mut cx = Cx::new_root();
    let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    let start = Instant::now();
    let root = build_widget(&mut cx, data, scroll_y, p);
    let _ = engine.compute(&mut cx, root, available);
    let _ = engine.result(root);
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn run_equivalence_checks(
    workload: &Workload,
    data: &[RowData],
    p: BuildParams,
) -> HashMap<String, EquivalenceCheck> {
    let mut out = HashMap::new();
    for check in &workload.equivalence_checks {
        let verdict = match check.name.as_str() {
            "visible_window_indices" => check_visible_window(data, p),
            _ => EquivalenceCheck::NotApplicable,
        };
        out.insert(check.name.clone(), verdict);
    }
    out
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn check_visible_window(data: &[RowData], p: BuildParams) -> EquivalenceCheck {
    Runtime::scope("sigil-bench-parity/visible-window-check", || {
        let target_row: f32 = 1_234.0;
        let scroll_y_val = target_row * p.row_height;
        let scroll_y = signal(scroll_y_val);
        let mut cx = Cx::new_root();
        let root = build_widget(&mut cx, data.to_vec(), scroll_y, p);
        let actual = visible_child_count(&cx, root);
        let expected_count = (p.viewport_height / p.row_height).ceil() as usize + 1;
        let expected_count = expected_count.min(data.len());

        if actual == expected_count {
            EquivalenceCheck::Pass
        } else {
            EquivalenceCheck::Fail {
                detail: format!(
                    "visible child count: expected {expected_count}, got {actual} \
                     (scroll_y={scroll_y_val}, row_height={}, viewport_height={})",
                    p.row_height, p.viewport_height,
                ),
            }
        }
    })
}

fn visible_child_count(cx: &Cx, root: NodeId) -> usize {
    let Some(content) = cx.first_child(root) else {
        return 0;
    };
    let Some(visible) = cx.first_child(content) else {
        return 0;
    };
    cx.child_count(visible)
}
