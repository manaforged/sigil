#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
//! Interaction tests for `code_edit` — the layer screenshots can't reach.
//!
//! These drive the real widget through a real `LayoutEngine` + `EventRouter` and
//! assert the things that broke in the live app but passed a render golden:
//!   1. a long line must NOT grow the widget's box past its pane (the overflow that
//!      stole clicks from neighbouring panes — "can't interact with anything");
//!   2. a click must focus the surface and keys must then reach `on_key`.

// Dev-deps referenced so `unused_crate_dependencies` stays quiet.
use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
#[cfg(feature = "tui")]
use sigil_tui as _;
use sigil_widgets as _;
use wgpu as _;

use std::cell::Cell;
use std::rc::Rc;

use sigil::prelude::*;
use sigil_hal::{KeyEventInit, Point, PointerEventInit, ScrollEventInit};

/// A measure that reports a fixed *wide* box for any text, so a single line is far
/// wider than the test pane — the condition that triggered the overflow bug.
struct WideMeasure;

impl TextMeasure for WideMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(1600.0, 16.0)
    }
}

fn layout_in(cx: &mut Cx, root: NodeId, w: f32, h: f32) -> LayoutEngine {
    let mut engine = LayoutEngine::with_text_measure(Box::new(WideMeasure));
    engine.compute(cx, root, Size::fixed(w, h)).unwrap();
    engine
}

fn first_text_input(cx: &Cx, root: NodeId) -> NodeId {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if found.is_none() && node.a11y.role == Role::TextInput {
            found = Some(id);
        }
    });
    found.expect("a code_edit node with Role::TextInput")
}

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Down,
    }))
}

const fn scroll_at(x: f32, y: f32, dy: f32) -> ShellEvent {
    ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
        position: Point::new(x, y),
        delta: ScrollDelta::Pixels(Point::new(0.0, dy)),
        modifiers: ModifiersState::empty(),
        phase: ScrollPhase::Moved,
    }))
}

fn char_key(text: &str) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(text),
        physical: PhysicalKey::KeyX,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: Some(text.into()),
    }))
}

fn named_key(key: Key, modifiers: ModifiersState) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::Named(key),
        physical: PhysicalKey::KeyX,
        modifiers,
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

/// True if any node under `root` is the caret bar — the only node laid out at the caret
/// height (`row_h * 0.8` = 14.4px). Height is used rather than width because the caret's
/// width is on the flex main axis and gets shrunk under the test's wide measure, whereas
/// its height is stable. Lets a test assert the caret actually rendered.
fn caret_bar_present(cx: &Cx, layout: &LayoutEngine, root: NodeId) -> bool {
    let mut found = false;
    cx.visit_preorder(root, |id, _node| {
        if let Some(result) = layout.result(id)
            && (result.rect().height - 14.4).abs() < 0.5
        {
            found = true;
        }
    });
    found
}

#[test]
fn long_line_does_not_grow_the_box_past_its_pane() {
    sigil::reactive::Runtime::scope("code-edit-overflow", || {
        let pane_w = 580.0;
        let lines = signal(vec!["x".repeat(200)]); // one line far wider than the pane
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(pane_w, 400.0))
                .child(code_edit(lines, caret)),
        );
        let layout = layout_in(&mut cx, root, pane_w, 400.0);
        let editor = first_text_input(&cx, root);
        let result = layout.result(editor).unwrap();

        assert!(
            result.rect().width <= pane_w + 0.5,
            "code_edit box must clip to its {pane_w}px pane, not grow to the longest line; \
             got {}",
            result.rect().width,
        );
        assert!(
            result.clips(),
            "code_edit must clip overflow so a long line can't paint/hit outside its pane",
        );
    });
}

/// Regression: a line wider than the pane must keep its run at full intrinsic width, not
/// shrink it to the pane. A run that shrinks to the available width gets RE-WRAPPED at
/// paint time (the engine wraps to the box width), drawing extra visual lines that
/// overflow the rigid row and bleed onto the next — the "long lines overlap" half of the
/// editor bug that row-level `shrink(0)` does not catch (the row box stays correct; the
/// text inside spills). `WideMeasure` reports every line at 1600px, far past the 580px
/// pane, so the run's laid-out width must stay ~1600 (the container clips it), not ~580.
#[test]
fn long_line_run_keeps_intrinsic_width_so_paint_does_not_wrap() {
    sigil::reactive::Runtime::scope("code-edit-nowrap-run", || {
        let pane_w = 580.0;
        let lines = signal(vec!["x".repeat(200)]); // measured 1600px wide by WideMeasure
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(pane_w, 400.0))
                .child(code_edit(lines, caret)),
        );
        let layout = layout_in(&mut cx, root, pane_w, 400.0);

        // The run is the widest node in the tree (the editor box itself clips to the
        // pane). With the no-wrap fix it keeps its ~1600px intrinsic width; without it,
        // flex shrinks it toward the ~580px pane and paint then wraps it.
        let mut widest = 0.0_f32;
        cx.visit_preorder(root, |id, _node| {
            if let Some(r) = layout.result(id) {
                widest = widest.max(r.rect().width);
            }
        });
        assert!(
            widest > pane_w + 100.0,
            "run was shrunk to the pane ({widest} <= {pane_w}) — paint will re-wrap it and \
             the long line will overlap the next row; the run must keep intrinsic width",
        );
    });
}

#[test]
fn click_focuses_surface_then_keys_reach_on_key() {
    sigil::reactive::Runtime::scope("code-edit-focus", || {
        let lines = signal(vec!["abc".to_string()]);
        let caret = signal((0_usize, 0_usize));
        let keys_seen = Rc::new(Cell::new(0_u32));
        let recorder = Rc::clone(&keys_seen);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret).key(move |_event| {
                    recorder.set(recorder.get() + 1);
                })),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        let editor = first_text_input(&cx, root);
        let mut router = EventRouter::new();

        // Before any focus, a key must not reach the surface.
        router.route(&mut cx, &layout, char_key("a"));
        assert_eq!(
            keys_seen.get(),
            0,
            "unfocused surface must not receive keys"
        );

        // A click inside the surface focuses it...
        router.route(&mut cx, &layout, pointer_down(10.0, 10.0));
        assert_eq!(
            router.focus(),
            Some(editor),
            "clicking the code surface must focus it",
        );

        // ...and now keys reach on_key.
        router.route(&mut cx, &layout, char_key("x"));
        assert_eq!(
            keys_seen.get(),
            1,
            "a focused code surface must forward keys to on_key",
        );
    });
}

#[test]
fn caret_renders_even_when_caret_line_is_past_the_end() {
    sigil::reactive::Runtime::scope("code-edit-caret-clamp", || {
        // The signals can momentarily disagree (a stale caret after the doc shrank). The
        // caret line points past the only row; it must clamp onto that row, not vanish.
        let lines = signal(vec!["abc".to_string()]);
        let caret = signal((9_usize, 1_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret)),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        assert!(
            caret_bar_present(&cx, &layout, root),
            "an out-of-range caret line must clamp to a real row so the caret still draws",
        );
    });
}

#[test]
fn trapped_tab_reaches_on_key_and_does_not_move_focus() {
    sigil::reactive::Runtime::scope("code-edit-tab-trap", || {
        let lines = signal(vec!["abc".to_string()]);
        let caret = signal((0_usize, 0_usize));
        let keys_seen = Rc::new(Cell::new(0_u32));
        let recorder = Rc::clone(&keys_seen);
        let mut cx = Cx::new_root();
        // A sibling focusable so `focus_next` has somewhere to go — without the trap,
        // Tab would move focus here instead of reaching the editor.
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret).key(move |_event| {
                    recorder.set(recorder.get() + 1);
                }))
                .child(
                    container()
                        .role(Role::Button)
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(100.0, 20.0)),
                ),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        let editor = first_text_input(&cx, root);
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(editor));

        router.route(
            &mut cx,
            &layout,
            named_key(Key::Tab, ModifiersState::empty()),
        );

        assert_eq!(
            keys_seen.get(),
            1,
            "a trap-tab surface must receive the raw Tab (to indent), not lose it to focus",
        );
        assert_eq!(
            router.focus(),
            Some(editor),
            "plain Tab on a trap-tab surface must not move focus away",
        );
    });
}

#[test]
fn shift_tab_still_escapes_a_trap_tab_surface() {
    sigil::reactive::Runtime::scope("code-edit-tab-escape", || {
        let lines = signal(vec!["abc".to_string()]);
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                // An `on_key` handler makes this a real trap (traps_tab is conditional).
                .child(code_edit(lines, caret).key(move |_event| {}))
                .child(
                    container()
                        .role(Role::Button)
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(100.0, 20.0)),
                ),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        let editor = first_text_input(&cx, root);
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(editor));

        // Shift+Tab is the escape hatch — it must move focus OFF the surface so the
        // trap is never a keyboard trap (WCAG 2.1.2).
        router.route(&mut cx, &layout, named_key(Key::Tab, ModifiersState::SHIFT));

        assert!(
            router.focus().is_some() && router.focus() != Some(editor),
            "Shift+Tab must move focus off a trap-tab surface (no keyboard trap)",
        );
    });
}

#[test]
fn read_only_surface_does_not_trap_tab() {
    sigil::reactive::Runtime::scope("code-edit-readonly-tab", || {
        // A code_edit with NO on_key is a read-only surface; Tab must traverse focus
        // normally rather than being swallowed by the (now-conditional) trap.
        let lines = signal(vec!["abc".to_string()]);
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret))
                .child(
                    container()
                        .role(Role::Button)
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(100.0, 20.0)),
                ),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        let editor = first_text_input(&cx, root);
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(editor));

        router.route(
            &mut cx,
            &layout,
            named_key(Key::Tab, ModifiersState::empty()),
        );

        assert!(
            router.focus().is_some() && router.focus() != Some(editor),
            "a read-only code surface must let Tab traverse focus, not trap it",
        );
    });
}

#[test]
fn caret_visible_when_doc_exceeds_max_rows() {
    sigil::reactive::Runtime::scope("code-edit-max-rows", || {
        // A document far larger than the row cap with the caret deep inside it: the
        // rendered window must follow the caret so it is never hidden behind the cap.
        let lines = signal((0..2_500).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((2_400_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(580.0, 400.0))
                .child(code_edit(lines, caret)),
        );
        let layout = layout_in(&mut cx, root, 580.0, 400.0);
        assert!(
            caret_bar_present(&cx, &layout, root),
            "caret on line 2400 of a 2500-line doc must stay in the rendered window",
        );
    });
}

/// Regression: a file taller than the editor pane must NOT compress its rows.
///
/// The rows live in a `flex_column` bounded by the pane height; once they overflow it,
/// flex's default shrink would squeeze every row below `ROW_H` to fit — the lines pile
/// up and their text overlaps (worse the longer the file). `row_container` pins
/// `shrink(0.0)` so rows stay rigid and the column clips the overflow instead. This
/// asserts the invariant directly: consecutive row tops stay exactly `ROW_H` apart, no
/// matter how far the content overflows. Guards the "editor text overlaps itself" bug
/// that a render golden at a short doc length never catches.
#[test]
fn overflowing_rows_keep_full_height_and_do_not_compress() {
    sigil::reactive::Runtime::scope("code-edit-overflow-compress", || {
        // Pin the row height the test controls (via `row_height`) rather than mirroring
        // the widget's private default, so this asserts shrink behavior only — not an
        // incidental default-height change.
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0; // 11 rows fit; 60 rows overflow by ~5x.
        let lines = signal((0..60).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let editor_node = code_edit(lines, caret).row_height(ROW_H);
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(editor_node),
        );
        let layout = layout_in(&mut cx, root, 400.0, pane_h);

        // The rows live in the scrolling inner column — the viewport's single child —
        // not directly under the viewport. Read each row top in order and assert the step
        // never falls below ROW_H (compression would shrink it).
        let editor = first_text_input(&cx, root);
        let inner = cx.nth_child(editor, 0).expect("scroll body column");
        let count = cx.child_count(inner);
        assert!(
            count >= 12,
            "expected the overflowing rows to be laid out, got {count}"
        );
        let tops: Vec<f32> = (0..count)
            .filter_map(|i| cx.nth_child(inner, i))
            .filter_map(|row| layout.result(row).map(|r| r.rect().origin.y))
            .collect();
        for pair in tops.windows(2) {
            let step = pair[1] - pair[0];
            assert!(
                (step - ROW_H).abs() < 0.5,
                "rows compressed under overflow: step {step} != ROW_H {ROW_H} (tops={tops:?})",
            );
        }
    });
}

/// Perf + scroll regression: the editor must VIRTUALIZE (materialize only a window of
/// rows, not the whole file — re-solving every row each frame was the ~110ms stall), and
/// a wheel must move that window past the caret (free scroll, not caret-locked).
#[test]
fn wheel_virtualizes_and_scrolls_the_window_past_the_caret() {
    sigil::reactive::Runtime::scope("code-edit-virtualize", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0;
        let lines = signal((0..400).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize)); // caret parks on the first line
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(code_edit(lines, caret).row_height(ROW_H)),
        );
        let mut layout = layout_in(&mut cx, root, 400.0, pane_h);
        let editor = first_text_input(&cx, root);
        let inner = cx.nth_child(editor, 0).expect("scroll body column");

        // Virtualization: a 400-line doc must materialize only a window, never 400 rows.
        let materialized = cx.child_count(inner);
        assert!(
            (10..150).contains(&materialized),
            "editor must render a bounded window of a 400-line file, got {materialized} rows",
        );
        // The caret on line 0 is visible before scrolling.
        assert!(caret_bar_present(&cx, &layout, root));

        // Wheel down well past the caret (50 rows). Because follow only fires on caret
        // MOVEMENT, the window leaves the caret behind — proving free scrolling.
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, scroll_at(200.0, 100.0, -50.0 * ROW_H));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, pane_h))
            .unwrap();
        assert!(
            !caret_bar_present(&cx, &layout, root),
            "a wheel-scroll must move the window past the line-0 caret (free scroll)",
        );
    });
}

/// The virtualization budget tracks the REAL viewport height (`size_reactive`), not a
/// fixed guess: a short pane materializes few rows. A discriminator — a stuck default
/// (800px) budget would render ~50, so a small result proves the published height fed back.
#[test]
fn render_budget_tracks_the_published_viewport_height() {
    sigil::reactive::Runtime::scope("code-edit-size-reactive", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0; // ~11 visible rows — far below the 800px default
        let lines = signal((0..400).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(code_edit(lines, caret).row_height(ROW_H)),
        );
        let mut layout = layout_in(&mut cx, root, 400.0, pane_h);
        // Drive real frames so `size_reactive` publishes the 200px height during attach and
        // the next Effect phase rebuilds the window at the now-smaller budget.
        for _ in 0..3 {
            sigil_app::__drive_test_frame(
                || {
                    layout
                        .compute(&mut cx, root, Size::fixed(400.0, pane_h))
                        .unwrap();
                },
                || {},
            );
        }
        let editor = first_text_input(&cx, root);
        let inner = cx.nth_child(editor, 0).expect("scroll body column");
        let materialized = cx.child_count(inner);
        // ~11 visible + 6 buffer ≈ 17, and crucially well under the ~50 a stuck 1600px
        // default would render — that gap is the proof the real height fed back.
        assert!(
            (10..40).contains(&materialized),
            "budget must shrink to the 200px pane via size_reactive, got {materialized}"
        );
    });
}

/// Regression (adversarial review M1): a document that FITS the pane must not scroll into
/// blank space. `max_scroll` is now `(content - viewport).max(0)` using the real height
/// from `size_reactive`, so a wheel on a short doc is a no-op (was `(lines-1)*row_h`, which
/// let a 5-line doc scroll 4 rows up, revealing blank below).
#[test]
fn short_doc_does_not_scroll_into_blank_space() {
    sigil::reactive::Runtime::scope("code-edit-shortdoc", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0; // fits ~11 rows
        let lines = signal((0..5).map(|i| format!("line {i}")).collect::<Vec<_>>()); // 5 < 11
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(code_edit(lines, caret).row_height(ROW_H)),
        );
        let mut layout = layout_in(&mut cx, root, 400.0, pane_h);
        // Publish the real 200px height so max_scroll knows the doc fits.
        for _ in 0..3 {
            sigil_app::__drive_test_frame(
                || {
                    layout
                        .compute(&mut cx, root, Size::fixed(400.0, pane_h))
                        .unwrap();
                },
                || {},
            );
        }
        assert!(
            caret_bar_present(&cx, &layout, root),
            "line-0 caret shows before scroll"
        );

        // A hard wheel-down must do nothing: the doc fits, so there's nowhere to scroll.
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, scroll_at(200.0, 100.0, -50.0 * ROW_H));
        layout
            .compute(&mut cx, root, Size::fixed(400.0, pane_h))
            .unwrap();
        assert!(
            caret_bar_present(&cx, &layout, root),
            "a doc that fits the pane must not scroll line 0 off into blank space",
        );
    });
}
