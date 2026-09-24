#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::hal::{KeyEventInit, Point, PointerEventInit, ScrollEventInit};
use sigil::prelude::*;

struct WideMeasure;

impl TextMeasure for WideMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(1600.0, 16.0)
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
        let lines = signal(vec!["x".repeat(200)]);
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

#[test]
fn long_line_run_keeps_intrinsic_width_so_paint_does_not_wrap() {
    sigil::reactive::Runtime::scope("code-edit-nowrap-run", || {
        let pane_w = 580.0;
        let lines = signal(vec!["x".repeat(200)]);
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(pane_w, 400.0))
                .child(code_edit(lines, caret)),
        );
        let layout = layout_in(&mut cx, root, pane_w, 400.0);

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

        router.route(&mut cx, &layout, char_key("a"));
        assert_eq!(
            keys_seen.get(),
            0,
            "unfocused surface must not receive keys"
        );

        router.route(&mut cx, &layout, pointer_down(10.0, 10.0));
        assert_eq!(
            router.focus(),
            Some(editor),
            "clicking the code surface must focus it",
        );

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

#[test]
fn overflowing_rows_keep_full_height_and_do_not_compress() {
    sigil::reactive::Runtime::scope("code-edit-overflow-compress", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0;
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

#[test]
fn wheel_virtualizes_and_scrolls_the_window_past_the_caret() {
    sigil::reactive::Runtime::scope("code-edit-virtualize", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0;
        let lines = signal((0..400).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(code_edit(lines, caret).row_height(ROW_H)),
        );
        let mut layout = layout_in(&mut cx, root, 400.0, pane_h);
        let editor = first_text_input(&cx, root);
        let inner = cx.nth_child(editor, 0).expect("scroll body column");

        let materialized = cx.child_count(inner);
        assert!(
            (10..150).contains(&materialized),
            "editor must render a bounded window of a 400-line file, got {materialized} rows",
        );
        assert!(caret_bar_present(&cx, &layout, root));

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

#[test]
fn render_budget_tracks_the_published_viewport_height() {
    sigil::reactive::Runtime::scope("code-edit-size-reactive", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0;
        let lines = signal((0..400).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(code_edit(lines, caret).row_height(ROW_H)),
        );
        let mut layout = layout_in(&mut cx, root, 400.0, pane_h);
        for _ in 0..3 {
            sigil::app::__drive_test_frame(
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
        assert!(
            (10..40).contains(&materialized),
            "budget must shrink to the 200px pane via size_reactive, got {materialized}"
        );
    });
}

#[test]
fn short_doc_does_not_scroll_into_blank_space() {
    sigil::reactive::Runtime::scope("code-edit-shortdoc", || {
        const ROW_H: f32 = 18.0;
        let pane_h = 200.0;
        let lines = signal((0..5).map(|i| format!("line {i}")).collect::<Vec<_>>());
        let caret = signal((0_usize, 0_usize));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(400.0, pane_h))
                .child(code_edit(lines, caret).row_height(ROW_H)),
        );
        let mut layout = layout_in(&mut cx, root, 400.0, pane_h);
        for _ in 0..3 {
            sigil::app::__drive_test_frame(
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
