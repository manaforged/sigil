#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;
use sigil_renderer::{SceneCommand, emit_scene};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(60.0, 16.0)
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
            virtual_list(items, |_, s: &String| container().child(text(s.clone())))
                .row_height(20.0)
                .viewport_height(200.0)
                .viewport_width(300.0),
        );

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .unwrap();
        assert_eq!(glyph_runs(&cx, root), 6, "initial bind paints all rows");

        items.set(vec!["match-1".to_string(), "match-2".to_string()]);
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .unwrap();
        assert_eq!(glyph_runs(&cx, root), 2, "shrunk bind still paints rows");
        let scene = emit_scene(&cx, root);
        assert!(
            scene.find_text("match-1").is_some(),
            "count change remounts index-keyed rows with new content"
        );
        assert!(
            scene.find_text("alpha").is_none(),
            "remount drops the previous slot occupant's content"
        );

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
            virtual_list(rows, |_, s: &String| container().child(text(s.clone())))
                .row_height(20.0)
                .viewport_height(200.0)
                .viewport_width(300.0),
        );

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .unwrap();
        assert_eq!(
            glyph_runs(&cx, root),
            13,
            "window of unfiltered rows paints"
        );

        for (typed, expect) in [("t", 13_usize), ("to", 13), ("tok", 13), ("token1", 11)] {
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
            virtual_list(rows, |_, s: &String| container().child(text(s.clone())))
                .row_height(20.0)
                .viewport_height(200.0)
                .viewport_width(300.0),
        );

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        let mut frame = |frame_cx: &mut Cx, write: Option<&str>| {
            Runtime::begin_effect();
            if let Some(s) = write {
                search.set(s.to_string());
            }
            Runtime::begin_layout();
            layout
                .compute(frame_cx, root, Size::fixed(640.0, 480.0))
                .unwrap();
            Runtime::begin_paint();
            let runs = glyph_runs(frame_cx, root);
            Runtime::end_frame();
            runs
        };

        assert_eq!(frame(&mut cx, None), 13, "unfiltered frame paints window");
        assert_eq!(frame(&mut cx, Some("t")), 13, "'t' frame paints window");
        assert_eq!(frame(&mut cx, Some("token1")), 11, "'token1' frame paints");
        assert_eq!(frame(&mut cx, None), 11, "idle follow-up frame keeps rows");
    });
}

#[test]
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
            virtual_list(rows, |i, s: &String| {
                container()
                    .layout(LayoutStyle::flex_row().fixed_size(900.0, 22.0).gap(8.0))
                    .child(text(format!("{i}")).class("text-on-surface text-xs"))
                    .child(text(s.clone()).class("text-on-surface text-xs font-mono"))
            })
            .row_height(22.0)
            .viewport_height(330.0)
            .viewport_width(952.0)
            .selection(SelectionMode::Single)
            .select(move |i| selected.set(Some(i))),
        );

        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        let mut frame = |frame_cx: &mut Cx, writes: &[&str]| {
            Runtime::begin_effect();
            for s in writes {
                search.set((*s).to_string());
            }
            Runtime::begin_layout();
            layout
                .compute(frame_cx, root, Size::fixed(1280.0, 800.0))
                .unwrap();
            Runtime::begin_paint();
            let runs = glyph_runs(frame_cx, root);
            Runtime::end_frame();
            runs
        };

        assert_eq!(frame(&mut cx, &[]), 16 * 2, "window of all rows paints");
        let shrunk = frame(&mut cx, &["t", "to", "tok", "toke", "token"]);
        assert_eq!(shrunk, 8 * 2, "batched shrink still paints rows");
        assert_eq!(frame(&mut cx, &[]), 8 * 2, "idle frame keeps rows");
    });
}

#[test]
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

        let mut ballast = container().layout(LayoutStyle::flex_column().fixed_size(300.0, 700.0));
        for i in 0..600 {
            ballast = ballast.child(
                container()
                    .layout(LayoutStyle::block().fixed_size(300.0, 1.0))
                    .child(text(format!("b{i}"))),
            );
        }

        let list = virtual_list(rows, |_, s: &String| container().child(text(s.clone())))
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
        let mut frame = |frame_cx: &mut Cx, write: Option<&str>| {
            Runtime::begin_effect();
            if let Some(s) = write {
                search.set(s.to_string());
            }
            Runtime::begin_layout();
            layout
                .compute(frame_cx, root, Size::fixed(1280.0, 800.0))
                .unwrap();
            Runtime::begin_paint();
            let runs = glyph_runs(frame_cx, root);
            Runtime::end_frame();
            runs
        };

        let ballast_runs = 600;
        assert_eq!(
            frame(&mut cx, None),
            ballast_runs + 19,
            "window of all rows paints next to ballast"
        );
        let after = frame(&mut cx, Some("token"));
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
