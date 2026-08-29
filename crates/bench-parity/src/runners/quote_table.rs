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
    SceneCount, check_visible, finish, first_from_scroll, last_measure_count, list_output,
    new_engine, paint, validate, verify_map, visible_row_count,
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
    profile::enable();
    profile::reset();
    let mut last: Option<(Cx, NodeId, SceneCount)> = None;
    let (samples_ns, misses) = {
        let mut step = || {
            let mut cx = Cx::new_root();
            let mut engine = new_engine(true);
            profile::begin_frame();
            let start = Instant::now();
            let root = cx.add(quote_list(rows, scroll).overscan(0));
            let _ = engine.compute(&mut cx, root, clock.available);
            let cmds = paint(&cx, root, clock.surface, clock.vw, clock.vh);
            profile::tick_frame();
            let missed = u64::from(engine.generation() > 0);
            last = Some((cx, root, cmds));
            (super::nanos(start), missed)
        };
        measure(p, &mut step)
    };
    let first = first_from_scroll(scroll.get(), p.row_h());
    finish_table(
        workload,
        samples_ns,
        misses,
        construct_evidence(last, book, first),
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
    let mut last_scene = SceneCount::default();
    let (samples_ns, misses) = {
        let mut step = || {
            bump_scroll(scroll, p.row_h(), max_scroll);
            let before = engine.generation();
            profile::begin_frame();
            let start = Instant::now();
            let _ = engine.compute(&mut cx, root, clock.available);
            last_scene = paint(&cx, root, clock.surface, clock.vw, clock.vh);
            profile::tick_frame();
            (super::nanos(start), u64::from(engine.generation() > before))
        };
        measure(p, &mut step)
    };
    let first = first_from_scroll(scroll.get(), p.row_h());
    let output = list_output(&cx, root, first);
    finish_table(
        workload,
        samples_ns,
        misses,
        TableEvidence {
            proof: Proof::from_tree(&cx, root, last_scene, book, false),
            identity: None,
            output: Some(output),
        },
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
    let mut last_scene = SceneCount::default();
    let mut prev = last_done_values(&cx, root);
    let mut moved = false;
    let (samples_ns, misses) = {
        let mut step = || {
            step_n = step_n.saturating_add(1);
            profile::begin_frame();
            let start = Instant::now();
            rows.update(|data| {
                tick_visible(Rc::make_mut(data).as_mut_slice(), 0, visible, step_n);
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
            (super::nanos(start), u64::from(engine.generation() > before))
        };
        measure(p, &mut step)
    };
    let held = !keep.is_empty() && keep == visible_slots(&cx, root);
    let output = list_output(&cx, root, 0);
    finish_table(
        workload,
        samples_ns,
        misses,
        TableEvidence {
            proof: Proof::from_tree(&cx, root, last_scene, book, moved),
            identity: Some(held),
            output: Some(output),
        },
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

impl Proof {
    fn from_tree(cx: &Cx, root: NodeId, scene: SceneCount, book: usize, moved: bool) -> Self {
        Self {
            visible: visible_row_count(cx, root),
            cells: visible_cell_count(cx, root),
            scene,
            book,
            last_done_moved: moved,
        }
    }
}

struct TableEvidence {
    proof: Proof,
    identity: Option<bool>,
    output: Option<crate::OutputProof>,
}

fn construct_evidence(
    last: Option<(Cx, NodeId, SceneCount)>,
    book: usize,
    first: usize,
) -> TableEvidence {
    let Some((cx, root, scene)) = last else {
        return TableEvidence {
            proof: Proof {
                visible: 0,
                cells: 0,
                scene: SceneCount::default(),
                book,
                last_done_moved: false,
            },
            identity: None,
            output: None,
        };
    };
    TableEvidence {
        proof: Proof::from_tree(&cx, root, scene, book, false),
        identity: None,
        output: Some(list_output(&cx, root, first)),
    }
}

fn finish_table(
    workload: &Workload,
    samples_ns: Vec<u64>,
    misses: u64,
    evidence: TableEvidence,
) -> crate::RunResult {
    let TableEvidence {
        proof,
        identity,
        output,
    } = evidence;
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
        scene_check(&workload.params, &proof, tick),
    );
    if let Some(held) = identity {
        extra.insert(
            "identity_did_hold".into(),
            pass_if(held, "visible row NodeIds changed on tick"),
        );
    }
    if tick {
        extra.insert(
            "last_done_did_move".into(),
            pass_if(proof.last_done_moved, "last_done resolve did not change"),
        );
    }
    finish(workload, samples_ns, extra, verify, output)
}

fn scene_check(p: &crate::WorkloadParams, proof: &Proof, tick: bool) -> EquivalenceCheck {
    scene_ok(
        p.surface(),
        proof.scene,
        in_view_rows(p.viewport().1, p.row_h(), proof.visible),
        tick,
        p.viewport().0,
    )
}

fn pass_if(ok: bool, detail: &str) -> EquivalenceCheck {
    if ok {
        EquivalenceCheck::Pass
    } else {
        EquivalenceCheck::Fail {
            detail: detail.into(),
        }
    }
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
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
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

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
fn max_y(rows: usize, row_h: f32, vh: f32) -> f32 {
    (rows as f32).mul_add(row_h, -vh)
}

#[cfg(test)]
#[path = "quote_table_tests.rs"]
mod tests;
