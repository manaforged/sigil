//! Repro for the capture-review dogfood finding: an `items_bind`
//! virtual list that shrinks (filter narrowing 676 → 7 rows) painted
//! nothing after the rebuild. Drives a persistent [`LayoutEngine`]
//! across signal mutations exactly like the app frame loop does.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use actions as _;
use animation as _;
use animation_runtime as _;
use app as _;
use arboard as _;
use criterion as _;
use events as _;
use hal as _;
use image as _;
use kernel as _;
use layout as _;
use pollster as _;
use reactive as _;
use style as _;
use tasks as _;
use wgpu as _;

use renderer::{SceneCommand, emit_scene};
use sigil::prelude::*;
use sigil::widgets::virtual_list;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: hal::Size) -> hal::LogicalSize {
        hal::LogicalSize::new(60.0, 16.0)
    }
}

fn glyph_runs(cx: &Cx, root: NodeId) -> usize {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter(|c| matches!(c, SceneCommand::GlyphRun(_)))
        .count()
}

#[test]
fn items_bind_shrink_still_paints_rows() {
    sigil::reactive::Runtime::scope("vl-bind-shrink", || {
        let mut cx = Cx::new_root();
        let items = signal(vec![
            "alpha".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
            "delta".to_string(),
            "epsilon".to_string(),
            "zeta".to_string(),
        ]);
        let root = cx.add(
            virtual_list(Vec::<String>::new(), |_, s: &String| {
                container().child(text(s.clone()))
            })
            .items_bind(items)
            .row_height(20.0)
            .viewport_height(200.0)
            .viewport_width(300.0),
        );

        // Persistent engine across frames, like the app loop.
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .unwrap();
        assert_eq!(glyph_runs(&cx, root), 6, "initial bind paints all rows");

        // Shrink — the filter-narrowing case.
        items.set(vec!["match-1".to_string(), "match-2".to_string()]);
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .unwrap();
        assert_eq!(glyph_runs(&cx, root), 2, "shrunk bind still paints rows");

        // Grow back.
        items.set(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ]);
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .unwrap();
        assert_eq!(glyph_runs(&cx, root), 4, "regrown bind still paints rows");
    });
}

/// Same shape as the capture-review app: the bound items signal is set
/// from inside ANOTHER effect (search → filter effect → `rows.set`),
/// and the search signal mutates per keystroke.
#[test]
fn items_bind_via_effect_chain_still_paints_rows() {
    sigil::reactive::Runtime::scope("vl-bind-effect-chain", || {
        let mut cx = Cx::new_root();
        let all: Vec<String> = (0..40).map(|i| format!("https://x/{i}/token{i}")).collect();
        let search = signal(String::new());
        let rows = signal(Vec::<String>::new());

        let source = all;
        effect(move || {
            let needle = search.get().to_lowercase();
            let next: Vec<String> = source
                .iter()
                .filter(|u| needle.is_empty() || u.to_lowercase().contains(&needle))
                .cloned()
                .collect();
            rows.set(next);
        });

        let root = cx.add(
            virtual_list(Vec::<String>::new(), |_, s: &String| {
                container().child(text(s.clone()))
            })
            .items_bind(rows)
            .row_height(20.0)
            .viewport_height(200.0)
            .viewport_width(300.0),
        );

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .unwrap();
        assert_eq!(glyph_runs(&cx, root), 40, "unfiltered rows paint");

        // Per-keystroke narrowing, layout between keystrokes like frames.
        for (typed, expect) in [("t", 40_usize), ("to", 40), ("tok", 40), ("token1", 11)] {
            search.set(typed.to_string());
            layout
                .compute(&mut cx, root, Size::fixed(640.0, 480.0))
                .unwrap();
            assert_eq!(
                glyph_runs(&cx, root),
                expect,
                "filter {typed:?} paints {expect} rows"
            );
        }
    });
}

/// Phase-accurate variant — drives the reactive runtime through the
/// production frame phases (Effect → Layout → Paint → end_frame) the
/// way `app::launch` does, with the keystroke write landing in the
/// Effect phase. This is the exact live-app shape.
#[test]
fn items_bind_shrink_paints_under_frame_phases() {
    sigil::reactive::Runtime::scope("vl-bind-phases", || {
        let mut cx = Cx::new_root();
        let all: Vec<String> = (0..40).map(|i| format!("https://x/{i}/token{i}")).collect();
        let search = signal(String::new());
        let rows = signal(Vec::<String>::new());

        let source = all;
        effect(move || {
            let needle = search.get().to_lowercase();
            let next: Vec<String> = source
                .iter()
                .filter(|u| needle.is_empty() || u.to_lowercase().contains(&needle))
                .cloned()
                .collect();
            rows.set(next);
        });

        let root = cx.add(
            virtual_list(Vec::<String>::new(), |_, s: &String| {
                container().child(text(s.clone()))
            })
            .items_bind(rows)
            .row_height(20.0)
            .viewport_height(200.0)
            .viewport_width(300.0),
        );

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        let mut frame = |cx: &mut Cx, write: Option<&str>| {
            Runtime::begin_effect();
            if let Some(s) = write {
                search.set(s.to_string());
            }
            Runtime::begin_layout();
            layout.compute(cx, root, Size::fixed(640.0, 480.0)).unwrap();
            Runtime::begin_paint();
            let runs = glyph_runs(cx, root);
            Runtime::end_frame();
            runs
        };

        assert_eq!(frame(&mut cx, None), 40, "unfiltered frame paints");
        assert_eq!(frame(&mut cx, Some("t")), 40, "'t' frame paints");
        assert_eq!(frame(&mut cx, Some("token1")), 11, "'token1' frame paints");
        assert_eq!(frame(&mut cx, None), 11, "idle follow-up frame keeps rows");
    });
}

/// Full capture-review shape: hundreds of multi-text rows, single
/// selection with a `.select` callback, and BATCHED keystrokes (several
/// signal writes inside one Effect phase — winit delivers fast typing
/// as multiple key events drained in a single frame).
#[test]
// One app-shaped scenario (build + batched keystrokes + assert), 58 lines;
// splitting would obscure the frame-batching behavior it pins.
#[allow(clippy::too_many_lines)]
fn items_bind_app_shape_batched_keystrokes() {
    sigil::reactive::Runtime::scope("vl-bind-app-shape", || {
        let mut cx = Cx::new_root();
        let all: Vec<String> = (0..676)
            .map(|i| {
                if i % 96 == 0 {
                    format!("https://x/auth/v1/token?n={i}")
                } else {
                    format!("https://x/asset/{i}.js")
                }
            })
            .collect();
        let search = signal(String::new());
        let rows = signal(Vec::<String>::new());
        let selected = signal(Option::<usize>::None);

        let source = all;
        effect(move || {
            let needle = search.get().to_lowercase();
            let next: Vec<String> = source
                .iter()
                .filter(|u| needle.is_empty() || u.to_lowercase().contains(&needle))
                .cloned()
                .collect();
            rows.set(next);
        });

        let root = cx.add(
            virtual_list(Vec::<String>::new(), |i, s: &String| {
                container()
                    .layout(LayoutStyle::flex_row().fixed_size(900.0, 22.0).gap(8.0))
                    .child(text(format!("{i}")).class("text-on-surface text-xs"))
                    .child(text(s.clone()).class("text-on-surface text-xs font-mono"))
            })
            .items_bind(rows)
            .row_height(22.0)
            .viewport_height(330.0)
            .viewport_width(952.0)
            .selection(SelectionMode::Single)
            .select(move |i| selected.set(Some(i))),
        );

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        let mut frame = |cx: &mut Cx, writes: &[&str]| {
            Runtime::begin_effect();
            for s in writes {
                search.set((*s).to_string());
            }
            Runtime::begin_layout();
            layout
                .compute(cx, root, Size::fixed(1280.0, 800.0))
                .unwrap();
            Runtime::begin_paint();
            let runs = glyph_runs(cx, root);
            Runtime::end_frame();
            runs
        };

        // 8 token rows × 2 texts each = 16 runs after filtering.
        assert_eq!(frame(&mut cx, &[]), 676 * 2, "all rows paint");
        let shrunk = frame(&mut cx, &["t", "to", "tok", "toke", "token"]);
        assert_eq!(shrunk, 8 * 2, "batched shrink still paints rows");
        assert_eq!(frame(&mut cx, &[]), 8 * 2, "idle frame keeps rows");
    });
}

/// Live-app divergence repro: a LARGE static sibling tree keeps the
/// post-rebuild dirty set under the broadcast threshold
/// (`dirty.len() < live/2`), forcing the layout engine down the
/// INCREMENTAL sync path. In the live capture-review app this path left
/// freshly rebuilt rows without layout — the list painted blank until a
/// window resize forced a full sync.
#[test]
// One shrink-repaint regression walk (84 lines); splitting the
// build/shrink/assert phases would obscure the incremental-sync bug it pins.
#[allow(clippy::too_many_lines)]
fn items_bind_shrink_paints_via_incremental_layout() {
    sigil::reactive::Runtime::scope("vl-bind-incremental", || {
        let mut cx = Cx::new_root();
        let all: Vec<String> = (0..30)
            .map(|i| {
                if i % 6 == 0 {
                    format!("https://x/auth/token?n={i}")
                } else {
                    format!("https://x/asset/{i}.js")
                }
            })
            .collect();
        let search = signal(String::new());
        let rows = signal(Vec::<String>::new());

        let source = all;
        effect(move || {
            let needle = search.get().to_lowercase();
            let next: Vec<String> = source
                .iter()
                .filter(|u| needle.is_empty() || u.to_lowercase().contains(&needle))
                .cloned()
                .collect();
            rows.set(next);
        });

        // Big static sibling subtree (like the session pane + detail
        // tabs around the live request list) so the rebuild's dirty set
        // stays far below live/2 and compute goes incremental.
        let mut ballast = container().layout(LayoutStyle::flex_column().fixed_size(300.0, 700.0));
        for i in 0..600 {
            ballast = ballast.child(
                container()
                    .layout(LayoutStyle::block().fixed_size(300.0, 1.0))
                    .child(text(format!("b{i}"))),
            );
        }

        let list = virtual_list(Vec::<String>::new(), |_, s: &String| {
            container().child(text(s.clone()))
        })
        .items_bind(rows)
        .row_height(20.0)
        .viewport_height(300.0)
        .viewport_width(400.0);

        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(900.0, 700.0))
                .child(ballast)
                .child(list),
        );

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        let mut frame = |cx: &mut Cx, write: Option<&str>| {
            Runtime::begin_effect();
            if let Some(s) = write {
                search.set(s.to_string());
            }
            Runtime::begin_layout();
            layout
                .compute(cx, root, Size::fixed(1280.0, 800.0))
                .unwrap();
            Runtime::begin_paint();
            let runs = glyph_runs(cx, root);
            Runtime::end_frame();
            runs
        };

        let ballast_runs = 600;
        assert_eq!(
            frame(&mut cx, None),
            ballast_runs + 30,
            "all rows paint next to ballast"
        );
        let after = frame(&mut cx, Some("token"));
        // Diagnostic: dump the token rows that actually painted + rects.
        for c in emit_scene(&cx, root).commands {
            if let SceneCommand::GlyphRun(d) = c
                && d.run.text.contains("token")
            {
                eprintln!("PAINTED {:?} @ y={}", d.run.text, d.run.rect.origin.y);
            }
        }
        assert_eq!(
            after,
            ballast_runs + 5,
            "incremental shrink still paints filtered rows"
        );
        assert_eq!(
            frame(&mut cx, None),
            ballast_runs + 5,
            "idle frame keeps filtered rows"
        );
    });
}
