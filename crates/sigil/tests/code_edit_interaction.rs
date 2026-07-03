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
use actions as _;
use animation as _;
use animation_runtime as _;
use app as _;
use arboard as _;
use criterion as _;
use events as _;
use image as _;
use kernel as _;
use pollster as _;
use renderer as _;
use style as _;
use tasks as _;
use wgpu as _;

use std::cell::Cell;
use std::rc::Rc;

use hal::{KeyEventInit, Point, PointerEventInit};
use sigil::prelude::*;

/// A measure that reports a fixed *wide* box for any text, so a single line is far
/// wider than the test pane — the condition that triggered the overflow bug.
struct WideMeasure;

impl TextMeasure for WideMeasure {
    fn measure(&self, _content: &TextContent, _space: hal::Size) -> hal::LogicalSize {
        hal::LogicalSize::new(1600.0, 16.0)
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

fn pointer_down(x: f32, y: f32) -> ShellEvent {
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
