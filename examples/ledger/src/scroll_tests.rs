#![allow(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

use std::time::Instant;

use sigil::testing::with_cx;
use sigil_hal::Size;
use sigil_layout::{LayoutEngine, TextEngineMeasure};
use sigil_text::OwnedTextEngine;

use crate::App;
use crate::view::{ROW_H, VIEWPORT_H, WINDOW_H, WINDOW_W, app_tree};

#[test]
#[ignore = "headless scroll budget; run with --release -- --ignored --nocapture"]
fn scroll_frames_stay_under_budget() {
    with_cx("ledger-scroll", |cx| {
        let app = App::new(10_000);
        let root = cx.add(app_tree(cx, &app));
        let mut engine = LayoutEngine::with_text_measure(Box::new(TextEngineMeasure::new(
            OwnedTextEngine::thread(),
        )));
        let size = Size::fixed(WINDOW_W, WINDOW_H);
        engine.compute(cx, root, size).unwrap();
        sigil_profile::enable();
        sigil_profile::reset();
        let mut us = Vec::new();
        for frame in 0..60_u32 {
            app.live.tick();
            app.scroll.set(frame as f32 * ROW_H * 2.0);
            sigil_profile::begin_frame();
            let start = Instant::now();
            engine.compute(cx, root, size).unwrap();
            sigil_profile::tick_frame();
            us.push(start.elapsed().as_micros());
        }
        us.sort_unstable();
        println!(
            "scroll frames: p50 {} us, p95 {} us, max {} us",
            us[30], us[57], us[59]
        );
        println!("{}", sigil_profile::summary());
        let full = sigil_profile::frames()
            .iter()
            .filter(|f| f.counts.iter().any(|c| c.0 == "layout.full"))
            .count();
        println!("frames with layout.full: {full}");
        assert!(
            us[57] < 16_000,
            "p95 scroll frame {} us over budget",
            us[57]
        );
    });
}

#[test]
#[ignore = "headless tick budget; run with --release -- --ignored --nocapture"]
fn tick_frames_stay_incremental() {
    with_cx("ledger-tick-frames", |cx| {
        let app = App::new(10_000);
        let root = cx.add(app_tree(cx, &app));
        let mut engine = LayoutEngine::with_text_measure(Box::new(TextEngineMeasure::new(
            OwnedTextEngine::thread(),
        )));
        let size = Size::fixed(WINDOW_W, WINDOW_H);
        engine.compute(cx, root, size).unwrap();
        sigil_profile::enable();
        sigil_profile::reset();
        let mut us = Vec::new();
        for _ in 0..60 {
            app.live.tick();
            sigil_profile::begin_frame();
            let start = Instant::now();
            engine.compute(cx, root, size).unwrap();
            sigil_profile::tick_frame();
            us.push(start.elapsed().as_micros());
        }
        us.sort_unstable();
        println!(
            "tick frames: p50 {} us, p95 {} us, max {} us",
            us[30], us[57], us[59]
        );
        println!("{}", sigil_profile::summary());
        let full = sigil_profile::frames()
            .iter()
            .filter(|f| f.counts.iter().any(|c| c.0 == "layout.full"))
            .count();
        println!("frames with layout.full: {full}");
    });
}

#[test]
#[ignore = "headless host frame budget; run with --release -- --ignored --nocapture"]
fn host_frames_stay_incremental() {
    use sigil::prelude::{Host, Runtime};
    sigil::testing::with_runtime("ledger-host-frames", |_| {
        let mut host = Host::new();
        let app = App::new(10_000);
        let tree = app.clone();
        host.build(move |cx| cx.add(app_tree(cx, &tree)));
        let viewport = (WINDOW_W, WINDOW_H);
        Runtime::begin_effect();
        Runtime::begin_layout();
        host.layout(viewport).unwrap();
        Runtime::begin_paint();
        let _ = host.painted();
        sigil_profile::enable();
        sigil_profile::reset();
        let mut us = Vec::new();
        for frame in 0..60_u32 {
            sigil_profile::begin_frame();
            let start = Instant::now();
            Runtime::begin_effect();
            app.live.tick();
            if frame % 2 == 0 {
                app.scroll.set(frame as f32 * ROW_H);
            }
            Runtime::begin_layout();
            let dirty = Runtime::frame_dirty();
            host.layout(viewport).unwrap();
            Runtime::begin_paint();
            let _ = host.painted();
            sigil_profile::tick_frame();
            us.push(start.elapsed().as_micros());
            if frame == 0 {
                println!("frame_dirty before layout: {dirty}");
            }
        }
        us.sort_unstable();
        println!(
            "host frames: p50 {} us, p95 {} us, max {} us",
            us[30], us[57], us[59]
        );
        println!("{}", sigil_profile::summary());
        let full = sigil_profile::frames()
            .iter()
            .filter(|f| f.counts.iter().any(|c| c.0 == "layout.full"))
            .count();
        println!("frames with layout.full: {full}");
    });
}

#[test]
#[ignore = "headless routed-event budget; run with --release -- --ignored --nocapture"]
fn routed_scroll_frames_stay_incremental() {
    use sigil::prelude::{EventRouter, Host, Runtime};
    use sigil_hal::{
        ModifiersState, MouseButtons, Point, PointerEvent, PointerEventInit, PointerKind,
        PointerPhase, ScrollDelta, ScrollEvent, ScrollEventInit, ScrollPhase, ShellEvent,
    };
    sigil::testing::with_runtime("ledger-routed-frames", |_| {
        let mut host = Host::new();
        let mut router = EventRouter::new();
        let app = App::new(10_000);
        let tree = app.clone();
        host.build(move |cx| cx.add(app_tree(cx, &tree)));
        let viewport = (WINDOW_W, WINDOW_H);
        Runtime::begin_effect();
        Runtime::begin_layout();
        host.layout(viewport).unwrap();
        Runtime::begin_paint();
        let _ = host.painted();
        let inside = Point::new(WINDOW_W / 2.0, WINDOW_H / 2.0);
        sigil_profile::enable();
        sigil_profile::reset();
        let mut us = Vec::new();
        for frame in 0..60_u32 {
            sigil_profile::begin_frame();
            let start = Instant::now();
            Runtime::begin_effect();
            app.live.tick();
            let wobble = Point::new(inside.x + (frame % 7) as f32, inside.y + (frame % 5) as f32);
            host.route(
                &mut router,
                ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
                    position: wobble,
                    delta: Point::new(1.0, 1.0),
                    kind: PointerKind::Mouse,
                    button: None,
                    buttons: MouseButtons::empty(),
                    modifiers: ModifiersState::empty(),
                    phase: PointerPhase::Move,
                })),
            );
            host.route(
                &mut router,
                ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
                    position: wobble,
                    delta: ScrollDelta::Pixels(Point::new(0.0, -ROW_H * 2.0)),
                    modifiers: ModifiersState::empty(),
                    phase: ScrollPhase::Moved,
                })),
            );
            Runtime::begin_layout();
            host.layout(viewport).unwrap();
            Runtime::begin_paint();
            let _ = host.painted();
            sigil_profile::tick_frame();
            us.push(start.elapsed().as_micros());
        }
        us.sort_unstable();
        println!(
            "routed frames: p50 {} us, p95 {} us, max {} us",
            us[30], us[57], us[59]
        );
        println!("{}", sigil_profile::summary());
        let full = sigil_profile::frames()
            .iter()
            .filter(|f| f.counts.iter().any(|c| c.0 == "layout.full"))
            .count();
        println!("frames with layout.full: {full}");
        println!("scroll signal after routing: {}", app.scroll.get());
    });
}

fn strip_offsets(cx: &sigil::prelude::Cx, root: sigil::prelude::NodeId) -> Vec<(f32, f32)> {
    use sigil_layout::{LayoutResult, ReactivePosition};
    let mut out = Vec::new();
    cx.visit_preorder(root, |id, node| {
        let Some(pos) = node.get_extension::<ReactivePosition>() else {
            return;
        };
        let Some(parent) = cx.parent(id) else {
            return;
        };
        let strip_y = node
            .get_extension::<LayoutResult>()
            .map(|r| r.rect().origin.y);
        let parent_y = cx
            .get(parent)
            .and_then(|p| p.get_extension::<LayoutResult>())
            .map(|r| r.rect().origin.y);
        if let (Some(s), Some(p)) = (strip_y, parent_y) {
            out.push((pos.get().1, s - p));
        }
    });
    out
}

#[test]
fn strip_position_tracks_scroll_through_ticks_and_rebuilds() {
    with_cx("ledger-strip-tracks", |cx| {
        let app = App::new(2_000);
        let root = cx.add(app_tree(cx, &app));
        let mut engine = LayoutEngine::with_text_measure(Box::new(TextEngineMeasure::new(
            OwnedTextEngine::thread(),
        )));
        let size = Size::fixed(WINDOW_W, WINDOW_H);
        engine.compute(cx, root, size).unwrap();
        sigil_profile::enable();
        sigil_profile::reset();
        for frame in 1..=40_u32 {
            app.live.tick();
            if frame % 2 == 0 {
                app.scroll.set(frame as f32 * ROW_H * 1.5);
            }
            sigil_profile::begin_frame();
            engine.compute(cx, root, size).unwrap();
            sigil_profile::tick_frame();
            for (wanted, got) in strip_offsets(cx, root) {
                assert!(
                    (wanted - got).abs() < 0.5,
                    "frame {frame}: strip inset {wanted} but laid out at {got}"
                );
            }
        }
    });
}

#[test]
#[ignore = "headless row build cost; run with --release -- --ignored --nocapture"]
fn row_build_cost_per_page_jump() {
    with_cx("ledger-row-build", |cx| {
        let app = App::new(10_000);
        let root = cx.add(app_tree(cx, &app));
        let mut engine = LayoutEngine::with_text_measure(Box::new(TextEngineMeasure::new(
            OwnedTextEngine::thread(),
        )));
        let size = Size::fixed(WINDOW_W, WINDOW_H);
        engine.compute(cx, root, size).unwrap();
        let page = (VIEWPORT_H / ROW_H).floor();
        let mut build = Vec::new();
        let mut layout = Vec::new();
        sigil_profile::enable();
        sigil_profile::reset();
        for jump in 1..=20_u32 {
            sigil_profile::begin_frame();
            let before = cx.len();
            app.scroll.set(jump as f32 * page * 3.0 * ROW_H);
            let t0 = Instant::now();
            cx.flush_reactive_children();
            let t1 = Instant::now();
            engine.compute(cx, root, size).unwrap();
            let t2 = Instant::now();
            let added = cx.len().saturating_sub(before);
            build.push(((t1 - t0).as_micros(), added));
            layout.push((t2 - t1).as_micros());
            sigil_profile::tick_frame();
        }
        println!("{}", sigil_profile::summary());
        let rows = (3.0 * (VIEWPORT_H / ROW_H)) as u128;
        let (bsum, nsum) = build
            .iter()
            .fold((0_u128, 0_usize), |(b, n), (us, added)| (b + us, n + added));
        println!(
            "page jumps: build {} us/jump ({} nodes/jump, {:.1} us/node), layout {} us/jump, ~{} rows mounted per jump",
            bsum / build.len() as u128,
            nsum / build.len(),
            bsum as f64 / nsum.max(1) as f64,
            layout.iter().sum::<u128>() / layout.len() as u128,
            rows
        );
    });
}
