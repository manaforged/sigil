//! GPUI framework runner — implements `FrameworkRunner` for the
//! `virtual-list` category against GPUI's `uniform_list` element.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bench_parity::{EquivalenceCheck, FrameworkRunner, HostInfo, RunResult, RunSummary, Workload};
use gpui::{
    AvailableSpace, Context, Div, IntoElement, ParentElement, Render, Styled, TestAppContext,
    TestDispatcher, UniformList, VisualTestContext, Window, div, point, px, size, uniform_list,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Cell widths for the krausest 4-cell row shape — kept in sync with
/// the Sigil runner so the two frameworks measure the same row layout.
const ID_CELL_W: f32 = 80.0;
const LABEL_CELL_W: f32 = 320.0;
const ACTION_CELL_W: f32 = 80.0;
const SPACER_CELL_W: f32 = 1600.0;
const ROW_WIDTH: f32 = ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W;

/// GPUI framework runner. Stateless.
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

    fn run(&self, workload: &Workload) -> Result<RunResult, String> {
        match workload.category.as_str() {
            "virtual-list" => run_virtual_list(workload),
            other => Err(format!(
                "GpuiRunner does not implement workload category {other:?}"
            )),
        }
    }
}

/// Empty root view registered on the test window. The actual bench
/// element is constructed inside the `VisualTestContext::draw` closure
/// so we can swap it per iteration without rebuilding the root entity.
struct EmptyRoot;

impl Render for EmptyRoot {
    fn render(&mut self, _w: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone, Copy)]
struct BuildParams {
    item_count: usize,
    row_height: f32,
    viewport_width: f32,
    viewport_height: f32,
}

fn run_virtual_list(workload: &Workload) -> Result<RunResult, String> {
    let p = &workload.params;
    if p.row_count == 0 {
        return Err("row_count must be > 0".into());
    }
    if p.iterations == 0 {
        return Err("iterations must be > 0".into());
    }
    let build = BuildParams {
        item_count: p.row_count,
        row_height: p.row_height_px,
        viewport_width: p.viewport_width_px,
        viewport_height: p.viewport_height_px,
    };

    // Silence the post-layout paint panic stderr noise. The panic is
    // expected and caught; the default hook would spam stderr with
    // one line per iteration and pollute the runner output.
    install_silent_panic_hook();

    let mut samples_ns: Vec<u64> = Vec::with_capacity(p.iterations);
    let mut last_visible: Option<Range<usize>> = None;

    for _ in 0..p.warmup {
        let (_elapsed, _ranges) = measure_one(build);
    }
    for _ in 0..p.iterations {
        let (elapsed, ranges) = measure_one(build);
        samples_ns.push(elapsed);
        if let Some(r) = last_real_range(&ranges) {
            last_visible = Some(r);
        }
    }

    let equivalence_check_results = run_equivalence_checks(workload, last_visible.as_ref(), build);

    Ok(RunResult {
        framework: "gpui".into(),
        workload_id: workload.workload_id.clone(),
        summary: RunSummary::from_samples(&samples_ns),
        samples_ns,
        equivalence_check_results,
        host: HostInfo::capture(),
    })
}

fn install_silent_panic_hook() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::panic::set_hook(Box::new(|_info| {}));
    });
}

/// Run a single frame and return (elapsed nanoseconds, every observed
/// render-range call). A fresh window is created per iteration because
/// the post-layout paint panic invalidates the previous one.
fn measure_one(p: BuildParams) -> (u64, Vec<Range<usize>>) {
    let dispatcher = TestDispatcher::new(StdRng::seed_from_u64(0));
    let mut cx = TestAppContext::build(dispatcher, Some("bench-parity-gpui"));
    let window = cx.add_window(|_w, _cx_view| EmptyRoot);
    let mut vc = VisualTestContext::from_window(window.into(), &cx);
    let observed: Arc<Mutex<Vec<Range<usize>>>> = Arc::new(Mutex::new(Vec::new()));
    let obs = observed.clone();
    let item_count = p.item_count;
    let row_h = p.row_height;
    let start = Instant::now();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        vc.draw(
            point(px(0.0), px(0.0)),
            size(px(p.viewport_width), px(p.viewport_height)).map(AvailableSpace::Definite),
            move |_w, _cx| build_uniform_list(item_count, row_h, obs),
        );
    }));
    let elapsed = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let ranges = observed.lock().map(|g| g.clone()).unwrap_or_default();
    (elapsed, ranges)
}

fn build_uniform_list(
    item_count: usize,
    row_h: f32,
    observed: Arc<Mutex<Vec<Range<usize>>>>,
) -> UniformList {
    uniform_list("bench-list", item_count, move |range, _w, _cx| {
        if let Ok(mut g) = observed.lock() {
            g.push(range.clone());
        }
        range.map(|_i| krausest_row(row_h)).collect::<Vec<Div>>()
    })
    .h_full()
    .w_full()
}

fn krausest_row(row_h: f32) -> Div {
    div()
        .h(px(row_h))
        .w(px(ROW_WIDTH))
        .child(div().h(px(row_h)).w(px(ID_CELL_W)))
        .child(div().h(px(row_h)).w(px(LABEL_CELL_W)))
        .child(div().h(px(row_h)).w(px(ACTION_CELL_W)))
        .child(div().h(px(row_h)).w(px(SPACER_CELL_W)))
}

/// GPUI invokes the render-range closure one or two times for internal
/// row-size probes (`0..1`); the genuine visible window is the last
/// call whose `range.len() > 1`.
fn last_real_range(ranges: &[Range<usize>]) -> Option<Range<usize>> {
    ranges.iter().rev().find(|r| r.end > r.start + 1).cloned()
}

fn run_equivalence_checks(
    workload: &Workload,
    last_visible: Option<&Range<usize>>,
    p: BuildParams,
) -> HashMap<String, EquivalenceCheck> {
    let mut out = HashMap::new();
    for check in &workload.equivalence_checks {
        let verdict = match check.name.as_str() {
            "visible_window_indices" => check_visible_window(last_visible, p),
            _ => EquivalenceCheck::NotApplicable,
        };
        out.insert(check.name.clone(), verdict);
    }
    out
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn check_visible_window(last_visible: Option<&Range<usize>>, p: BuildParams) -> EquivalenceCheck {
    let Some(range) = last_visible else {
        return EquivalenceCheck::Fail {
            detail: "GPUI uniform_list never invoked the render-range \
                     closure with a non-probe range; layout did not \
                     complete."
                .into(),
        };
    };
    let actual = range.end - range.start;
    let expected = (p.viewport_height / p.row_height).ceil() as usize + 1;
    let expected = expected.min(p.item_count);
    // Sigil's reference accepts an exact match against
    // `ceil(viewport_h / row_h) + 1`. GPUI computes its visible range
    // via `floor(scroll/h)..ceil((scroll+vh)/h)` — at scroll=0 this
    // produces `ceil(vh/h)` exactly (no +1 padding row). Accept ±1 to
    // span both inclusive-bound conventions.
    let lo = expected.saturating_sub(1);
    let hi = expected + 1;
    if actual >= lo && actual <= hi {
        EquivalenceCheck::Pass
    } else {
        EquivalenceCheck::Fail {
            detail: format!(
                "visible window length: expected ~{expected} (range {lo}..={hi}), \
                 got {actual} (range={range:?}, row_height={}, viewport_height={})",
                p.row_height, p.viewport_height,
            ),
        }
    }
}
