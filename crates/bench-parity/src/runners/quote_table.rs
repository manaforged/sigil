//! 24-column quote table — same `data_table::view::quote_list` as the window.

use std::rc::Rc;
use std::time::Instant;

use data_table::quote::Quote;
use data_table::view::quote_list;
use data_table::{COLUMN_COUNT, FIELDS, FieldKind, columns_intersecting, generate, tick_visible};
use sigil_hal::Size;
use sigil_kernel::{Cx, NodeId, NodeType};
use sigil_profile as profile;
use sigil_reactive::{Signal, signal};

use super::sigil_runner::{
    SceneCount, check_visible, finish, last_measure_count, new_engine, paint, validate, verify_map,
    visible_row_count,
};

use crate::{EquivalenceCheck, Surface, Workload, layout_void};

struct Clock {
    available: Size,
    surface: Surface,
    vw: f32,
    vh: f32,
}

pub(super) fn run(workload: &Workload) -> Result<crate::RunResult, String> {
    let p = &workload.params;
    validate(p.row_count, p.iterations)?;
    let (vw, vh) = p.viewport();
    let clock = Clock {
        available: Size::fixed(vw, vh),
        surface: p.surface(),
        vw,
        vh,
    };
    let quotes = Rc::new(generate(p.row_count));
    if p.tick() {
        Ok(tick_samples(workload, quotes, &clock))
    } else if p.reuse {
        Ok(scroll_samples(workload, quotes, &clock))
    } else {
        Ok(construct_samples(workload, quotes, &clock))
    }
}

fn construct_samples(
    workload: &Workload,
    quotes: Rc<Vec<Quote>>,
    clock: &Clock,
) -> crate::RunResult {
    let p = &workload.params;
    let book = quotes.len();
    let rows = signal(quotes);
    let scroll = signal(0.0_f32);
    let max_scroll = max_y(p.row_count, p.row_h(), clock.vh);
    profile::enable();
    profile::reset();
    let mut last: Option<(Cx, NodeId, SceneCount)> = None;
    let mut step = || {
        bump_scroll(scroll, p.row_h(), max_scroll);
        let mut cx = Cx::new_root();
        let mut engine = new_engine(true);
        profile::begin_frame();
        let start = Instant::now();
        let root = cx.add(quote_list(rows, scroll));
        let _ = engine.compute(&mut cx, root, clock.available);
        let cmds = paint(&cx, root, clock.surface, clock.vw, clock.vh);
        profile::tick_frame();
        let missed = u64::from(engine.generation() > 0);
        last = Some((cx, root, cmds));
        (nanos(start), missed)
    };
    let (samples_ns, misses) = measure(p, &mut step);
    let visible = last
        .as_ref()
        .map_or(0, |(cx, root, _)| visible_row_count(cx, *root));
    let cells = last
        .as_ref()
        .map_or(0, |(cx, root, _)| visible_cell_count(cx, *root));
    let scene = last.map_or(
        SceneCount {
            draws: 0,
            glyphs: 0,
        },
        |(_, _, s)| s,
    );
    finish_table(
        workload,
        samples_ns,
        misses,
        Proof {
            visible,
            cells,
            scene,
            book,
            last_done_moved: false,
        },
        None,
    )
}

fn scroll_samples(workload: &Workload, quotes: Rc<Vec<Quote>>, clock: &Clock) -> crate::RunResult {
    let p = &workload.params;
    let book = quotes.len();
    let rows = signal(quotes);
    let scroll = signal(0.0_f32);
    let max_scroll = max_y(p.row_count, p.row_h(), clock.vh);
    let mut cx = Cx::new_root();
    let mut engine = new_engine(true);
    let root = cx.add(quote_list(rows, scroll));
    let _ = engine.compute(&mut cx, root, clock.available);
    profile::enable();
    profile::reset();
    let mut last_scene = SceneCount {
        draws: 0,
        glyphs: 0,
    };
    let mut step = || {
        bump_scroll(scroll, p.row_h(), max_scroll);
        let before = engine.generation();
        profile::begin_frame();
        let start = Instant::now();
        let _ = engine.compute(&mut cx, root, clock.available);
        last_scene = paint(&cx, root, clock.surface, clock.vw, clock.vh);
        profile::tick_frame();
        (nanos(start), u64::from(engine.generation() > before))
    };
    let (samples_ns, misses) = measure(p, &mut step);
    finish_table(
        workload,
        samples_ns,
        misses,
        Proof {
            visible: visible_row_count(&cx, root),
            cells: visible_cell_count(&cx, root),
            scene: last_scene,
            book,
            last_done_moved: false,
        },
        None,
    )
}

fn tick_samples(workload: &Workload, quotes: Rc<Vec<Quote>>, clock: &Clock) -> crate::RunResult {
    let p = &workload.params;
    let book = quotes.len();
    let rows = signal(quotes);
    let scroll = signal(0.0_f32);
    let mut cx = Cx::new_root();
    let mut engine = new_engine(true);
    let root = cx.add(quote_list(rows, scroll));
    let _ = engine.compute(&mut cx, root, clock.available);
    let keep = visible_slots(&cx, root);
    let visible = visible_row_count(&cx, root);
    let mut step_n = 0_u32;
    profile::enable();
    profile::reset();
    let mut last_scene = SceneCount {
        draws: 0,
        glyphs: 0,
    };
    let mut prev = last_done_values(&cx, root);
    let mut moved = false;
    let mut step = || {
        step_n = step_n.saturating_add(1);
        profile::begin_frame();
        let start = Instant::now();
        rows.update(|book| {
            tick_visible(Rc::make_mut(book).as_mut_slice(), 0, visible, step_n);
        });
        let before = engine.generation();
        let _ = engine.compute(&mut cx, root, clock.available);
        let now = last_done_values(&cx, root);
        last_scene = paint(&cx, root, clock.surface, clock.vw, clock.vh);
        profile::tick_frame();
        if !now.is_empty() && now != prev {
            moved = true;
        }
        prev = now;
        (nanos(start), u64::from(engine.generation() > before))
    };
    let (samples_ns, misses) = measure(p, &mut step);
    let held = !keep.is_empty() && keep == visible_slots(&cx, root);
    finish_table(
        workload,
        samples_ns,
        misses,
        Proof {
            visible: visible_row_count(&cx, root),
            cells: visible_cell_count(&cx, root),
            scene: last_scene,
            book,
            last_done_moved: moved,
        },
        Some(held),
    )
}

fn visible_slots(cx: &Cx, root: NodeId) -> Vec<NodeId> {
    let Some(window) = first3(cx, root) else {
        return Vec::new();
    };
    (0..cx.child_count(window))
        .filter_map(|i| cx.nth_child(window, i))
        .collect()
}

fn measure(p: &crate::WorkloadParams, step: &mut impl FnMut() -> (u64, u64)) -> (Vec<u64>, u64) {
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

#[derive(Clone, Copy)]
struct Proof {
    visible: usize,
    cells: usize,
    scene: SceneCount,
    book: usize,
    last_done_moved: bool,
}

fn finish_table(
    workload: &Workload,
    samples_ns: Vec<u64>,
    misses: u64,
    proof: Proof,
    identity: Option<bool>,
) -> crate::RunResult {
    let tick = workload.params.tick();
    let mut verify = verify_map(
        samples_ns.len(),
        misses,
        u64::try_from(proof.visible).unwrap_or(0),
        last_measure_count(),
    );
    verify.insert("scene_cmds_last".into(), proof.scene.draws);
    verify.insert("scene_glyphs_last".into(), proof.scene.glyphs);
    verify.insert("book_rows".into(), u64::try_from(proof.book).unwrap_or(0));
    if tick {
        verify.insert("tick".into(), 1);
    }
    let mut extra = check_visible(proof.visible, &workload.params);
    extra.insert(
        "book_held".into(),
        crate::book_held(proof.book, workload.params.row_count),
    );
    extra.insert(
        "column_count".into(),
        columns_ok(proof.visible, proof.cells),
    );
    extra.insert("layout_did_work".into(), work_verdict(&verify, tick));
    extra.insert(
        "text_did_work".into(),
        measure_ok(proof.visible, &verify, workload.params.reuse, tick),
    );
    extra.insert(
        "scene_did_work".into(),
        scene_ok(
            workload.params.surface(),
            proof.scene,
            in_view_rows(
                workload.params.viewport().1,
                workload.params.row_h(),
                proof.visible,
            ),
            tick,
            workload.params.viewport().0,
        ),
    );
    if let Some(held) = identity {
        extra.insert(
            "identity_did_hold".into(),
            if held {
                EquivalenceCheck::Pass
            } else {
                EquivalenceCheck::Fail {
                    detail: "visible row NodeIds changed on tick".into(),
                }
            },
        );
    }
    if tick {
        extra.insert(
            "last_done_did_move".into(),
            if proof.last_done_moved {
                EquivalenceCheck::Pass
            } else {
                EquivalenceCheck::Fail {
                    detail: "last_done resolve did not change".into(),
                }
            },
        );
    }
    finish(workload, samples_ns, extra, verify)
}

fn last_done_col() -> usize {
    FIELDS
        .iter()
        .position(|field| matches!(field.kind, FieldKind::LastDone))
        .unwrap_or(3)
}

fn last_done_values(cx: &Cx, root: NodeId) -> Vec<String> {
    let Some(window) = first3(cx, root) else {
        return Vec::new();
    };
    let col = last_done_col();
    let mut out = Vec::new();
    for i in 0..cx.child_count(window) {
        let Some(slot) = cx.nth_child(window, i) else {
            continue;
        };
        let row = row_of(cx, slot);
        let Some(cell) = cx.nth_child(row, col) else {
            continue;
        };
        if let Some(value) = resolve_text(cx, cell) {
            out.push(value);
        }
    }
    out
}

fn resolve_text(cx: &Cx, id: NodeId) -> Option<String> {
    let node = cx.get(id)?;
    if let NodeType::Text { content, .. } = &node.kind {
        return Some(content.resolve());
    }
    resolve_text(cx, cx.first_child(id)?)
}

fn measure_ok(
    visible: usize,
    verify: &std::collections::HashMap<String, u64>,
    reuse: bool,
    tick: bool,
) -> EquivalenceCheck {
    let got = verify.get("text_measure_last").copied().unwrap_or(0);
    let band = u64::try_from(visible.saturating_mul(COLUMN_COUNT)).unwrap_or(0);
    let cols = u64::try_from(COLUMN_COUNT).unwrap_or(0);
    if tick {
        return if visible > 0 && got < band {
            EquivalenceCheck::Pass
        } else {
            EquivalenceCheck::Fail {
                detail: format!("tick text.measure {got} >= {band} (remasured the band)"),
            }
        };
    }
    if !reuse {
        return if visible > 0 && got >= band {
            EquivalenceCheck::Pass
        } else {
            EquivalenceCheck::Fail {
                detail: format!("text.measure {got} < {band} (visible {visible} × {COLUMN_COUNT})"),
            }
        };
    }
    if visible > 0 && got >= cols && got < band {
        EquivalenceCheck::Pass
    } else {
        EquivalenceCheck::Fail {
            detail: format!(
                "reuse text.measure {got} not in {cols}..{band} (one new row, not the band)"
            ),
        }
    }
}

fn scene_ok(
    surface: Surface,
    scene: SceneCount,
    painted: usize,
    tick: bool,
    viewport_w: f32,
) -> EquivalenceCheck {
    // `painted` is the in-view page. Overscan is mounted; clip hides it.
    let rows = u64::try_from(painted).unwrap_or(0);
    let cols = u64::try_from(columns_intersecting(viewport_w)).unwrap_or(0);
    let min_glyphs = if tick {
        rows
    } else {
        rows.saturating_mul(cols)
    };
    match surface {
        Surface::Layout => EquivalenceCheck::NotApplicable,
        Surface::LayoutPaint
            if painted > 0 && scene.glyphs >= min_glyphs && scene.draws >= rows =>
        {
            EquivalenceCheck::Pass
        }
        Surface::LayoutPaint => EquivalenceCheck::Fail {
            detail: format!(
                "emit glyphs {} < {min_glyphs} (page {painted}, clip cols {cols})",
                scene.glyphs
            ),
        },
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn in_view_rows(viewport_h: f32, row_h: f32, visible: usize) -> usize {
    if row_h <= 0.0 {
        return visible;
    }
    ((viewport_h / row_h).ceil() as usize).max(1).min(visible)
}

fn columns_ok(visible: usize, cells: usize) -> EquivalenceCheck {
    let expect = visible.saturating_mul(COLUMN_COUNT);
    if visible > 0 && cells == expect {
        EquivalenceCheck::Pass
    } else {
        EquivalenceCheck::Fail {
            detail: format!("cells {cells} != {visible} × {COLUMN_COUNT}"),
        }
    }
}

fn visible_cell_count(cx: &Cx, root: NodeId) -> usize {
    let Some(window) = first3(cx, root) else {
        return 0;
    };
    let mut cells = 0;
    for i in 0..cx.child_count(window) {
        let Some(slot) = cx.nth_child(window, i) else {
            continue;
        };
        // virtual_list wraps each render() row in a slot.
        let row = row_of(cx, slot);
        cells += cx.child_count(row);
    }
    cells
}

fn row_of(cx: &Cx, slot: NodeId) -> NodeId {
    let Some(inner) = cx.first_child(slot) else {
        return slot;
    };
    if cx.child_count(slot) == 1 && cx.child_count(inner) > 0 {
        inner
    } else {
        slot
    }
}

fn first3(cx: &Cx, root: NodeId) -> Option<NodeId> {
    let host = cx.first_child(root)?;
    let content = cx.first_child(host)?;
    cx.first_child(content)
}

fn work_verdict(verify: &std::collections::HashMap<String, u64>, tick: bool) -> EquivalenceCheck {
    if tick {
        return EquivalenceCheck::Pass;
    }
    if layout_void(verify) {
        EquivalenceCheck::Fail {
            detail: "layout_hits == samples — timer did not remasure".into(),
        }
    } else {
        EquivalenceCheck::Pass
    }
}

fn bump_scroll(scroll: Signal<f32>, step: f32, max_scroll: f32) {
    scroll.update(|value| {
        *value += step;
        if *value > max_scroll {
            *value = 0.0;
        }
    });
}

#[allow(clippy::cast_precision_loss)]
fn max_y(rows: usize, row_h: f32, vh: f32) -> f32 {
    (rows as f32).mul_add(row_h, -vh)
}

fn nanos(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use crate::{
        EquivalenceCheck, EquivalenceCheckSpec, FrameworkRunner, SigilRunner, Workload,
        WorkloadParams, layout_void,
    };

    fn tiny(reuse: bool) -> Workload {
        Workload {
            workload_id: "data-table-smoke".into(),
            description: "anti-void smoke".into(),
            category: "data-table".into(),
            params: WorkloadParams {
                row_count: 200,
                row_height_px: 24.0,
                viewport_width_px: 1248.0,
                viewport_height_px: 912.0,
                row_shape: "gpui-24-field".into(),
                scroll_pattern: "linear".into(),
                iterations: 4,
                warmup: 1,
                surface: "layout".into(),
                reuse,
                cells_per_row: 24,
            },
            equivalence_checks: vec![
                spec("visible_window_indices"),
                spec("column_count"),
                spec("layout_did_work"),
                spec("text_did_work"),
            ],
        }
    }

    fn spec(name: &str) -> EquivalenceCheckSpec {
        EquivalenceCheckSpec {
            name: name.into(),
            description: String::new(),
        }
    }

    fn assert_real(reuse: bool) {
        let result = SigilRunner::new().run(&tiny(reuse)).expect("runner");
        assert!(
            !layout_void(&result.verify),
            "void run reuse={reuse} verify={:?}",
            result.verify
        );
        assert!(
            matches!(
                result.equivalence_check_results.get("layout_did_work"),
                Some(EquivalenceCheck::Pass)
            ),
            "layout_did_work {:?}",
            result.equivalence_check_results
        );
        assert!(
            matches!(
                result
                    .equivalence_check_results
                    .get("visible_window_indices"),
                Some(EquivalenceCheck::Pass)
            ),
            "visible {:?}",
            result.equivalence_check_results
        );
        assert!(
            matches!(
                result.equivalence_check_results.get("column_count"),
                Some(EquivalenceCheck::Pass)
            ),
            "column_count {:?}",
            result.equivalence_check_results
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

    #[test]
    fn construct_is_not_a_cache_hit() {
        assert_real(false);
    }

    #[test]
    fn scroll_reuse_is_not_a_cache_hit() {
        assert_real(true);
    }

    #[test]
    fn tick_keeps_identity_and_skips_band_measure() {
        let mut workload = tiny(true);
        workload.params.scroll_pattern = String::from("tick-visible");
        workload.params.surface = String::from("layout_paint");
        workload.equivalence_checks.push(spec("identity_did_hold"));
        let result = SigilRunner::new().run(&workload).expect("runner");
        assert!(
            matches!(
                result.equivalence_check_results.get("identity_did_hold"),
                Some(EquivalenceCheck::Pass)
            ),
            "identity {:?}",
            result.equivalence_check_results
        );
        assert!(
            matches!(
                result.equivalence_check_results.get("text_did_work"),
                Some(EquivalenceCheck::Pass)
            ),
            "text_did_work {:?}",
            result.equivalence_check_results
        );
        assert!(
            matches!(
                result.equivalence_check_results.get("scene_did_work"),
                Some(EquivalenceCheck::Pass)
            ),
            "scene_did_work {:?}",
            result.equivalence_check_results
        );
        assert!(
            matches!(
                result.equivalence_check_results.get("last_done_did_move"),
                Some(EquivalenceCheck::Pass)
            ),
            "last_done {:?}",
            result.equivalence_check_results
        );
    }

    #[test]
    fn remount_paint_emits_row_draws() {
        let mut workload = tiny(true);
        workload.params.surface = String::from("layout_paint");
        let result = SigilRunner::new().run(&workload).expect("runner");
        assert!(
            matches!(
                result.equivalence_check_results.get("scene_did_work"),
                Some(EquivalenceCheck::Pass)
            ),
            "scene_did_work {:?}",
            result.equivalence_check_results
        );
        assert!(
            result.verify.get("scene_cmds_last").copied().unwrap_or(0) >= 6,
            "verify={:?}",
            result.verify
        );
    }
}
