//! `command_menu` — state-machine + visual-golden tests for the FFT-style
//! nested action menu (HD-2D Task 9).
//!
//! Logic tests drive keyboard input through the same `EventRouter` +
//! `LayoutEngine` pipeline the sibling widget tests use (see
//! `modal_trap.rs` / `slider_keyboard.rs`), re-running `layout.compute`
//! after every key so the widget's `children_reactive` navigation stack
//! is flushed before the next assertion (see `sigil_kernel::Cx::build`'s
//! `children_reactive` doc: the rebuild queue only drains inside
//! `LayoutEngine::compute`).
//!
//! Goldens reuse `golden_support` directly, mirroring
//! `visual_goldens_extra.rs`. The "nested" and "disabled item" goldens
//! pre-navigate the widget with a private key-driven helper *inside*
//! the `build` closure — the shared harness's `InteractionMode` only
//! covers hover/press/focus, not arbitrary key sequences, so the
//! navigation happens before `golden_support::render_widget_to_png`
//! takes over the (idempotent) second layout pass.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::doc_markdown, clippy::too_many_lines, clippy::print_stdout)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use tiny_skia as _;
use wgpu as _;

use sigil::prelude::*;
use sigil_hal::{KeyEventInit, PhysicalKey};

mod golden_support;

use golden_support::{
    InteractionMode, assert_png_matches_golden, golden_path, render_widget_to_png,
};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

const VIEWPORT: (f32, f32) = (420.0, 260.0);

fn key_down(key: Key, physical: PhysicalKey) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: key.into(),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase: KeyPhase::Down,
        text: None,
    }))
}

/// A representative menu: a leaf, a two-item submenu with one disabled
/// entry, a disabled root row, and a trailing leaf.
fn sample_menu(select: impl Fn(&str) + 'static) -> impl FnOnce() -> CommandMenuBuilder + 'static {
    move || {
        command_menu()
            .item("Attack", || {})
            .submenu("Ability", |m| {
                m.item("Fire", || {}).item("Cure", || {}).disabled("Ultima")
            })
            .disabled("Item")
            .item("Wait", || {})
            .select(select)
    }
}

// ---- logic: EventRouter + LayoutEngine harness -----------------------------

fn setup(build: impl FnOnce() -> CommandMenuBuilder) -> (Cx, LayoutEngine, EventRouter, NodeId) {
    let mut cx = Cx::new_root();
    let root = cx.add(build());
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(&mut cx, root, Size::fixed(VIEWPORT.0, VIEWPORT.1))
        .expect("layout");
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(root));
    (cx, layout, router, root)
}

fn press(
    cx: &mut Cx,
    layout: &mut LayoutEngine,
    router: &mut EventRouter,
    root: NodeId,
    key: (Key, PhysicalKey),
) {
    router.route(cx, layout, key_down(key.0, key.1));
    layout
        .compute(cx, root, Size::fixed(VIEWPORT.0, VIEWPORT.1))
        .expect("layout after key");
}

fn panel_count(cx: &Cx, root: NodeId) -> usize {
    cx.get(root).expect("command menu root").children().len()
}

#[test]
fn nested_submenu_opens_and_esc_ascends() {
    Runtime::scope("cmd-menu-nested-esc", || {
        let (mut cx, mut layout, mut router, root) = setup(|| {
            command_menu()
                .item("Attack", || {})
                .submenu("Ability", |m| m.item("Fire", || {}))
        });

        assert_eq!(panel_count(&cx, root), 1, "starts with only the root panel");

        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (Key::ArrowDown, PhysicalKey::ArrowDown),
        );
        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (Key::Enter, PhysicalKey::Enter),
        );
        assert_eq!(
            panel_count(&cx, root),
            2,
            "descending into Ability opens a second panel"
        );

        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (Key::Escape, PhysicalKey::Escape),
        );
        assert_eq!(
            panel_count(&cx, root),
            1,
            "esc ascends back to the root panel"
        );
    });
}

#[test]
fn disabled_item_skipped_by_navigation() {
    Runtime::scope("cmd-menu-disabled-skip", || {
        let selected: Signal<Vec<String>> = signal(Vec::new());
        let (mut cx, mut layout, mut router, root) = setup(move || {
            command_menu()
                .item("A", || {})
                .disabled("B")
                .item("C", || {})
                .select(move |label| selected.update(|v| v.push(label.to_owned())))
        });

        // From "A", ArrowDown must skip disabled "B" and land on "C".
        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (Key::ArrowDown, PhysicalKey::ArrowDown),
        );
        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (Key::Enter, PhysicalKey::Enter),
        );

        assert_eq!(selected.get(), vec!["C".to_owned()]);
    });
}

#[test]
fn cancel_fires_on_root_esc() {
    Runtime::scope("cmd-menu-cancel", || {
        let cancelled = signal(false);
        let (mut cx, mut layout, mut router, root) = setup(move || {
            command_menu()
                .item("Attack", || {})
                .cancel(move || cancelled.set(true))
        });

        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (Key::Escape, PhysicalKey::Escape),
        );
        assert!(cancelled.get(), "esc at the root level must fire cancel");
    });
}

#[test]
fn leaf_select_fires_observer_and_closes() {
    Runtime::scope("cmd-menu-select-closes", || {
        let fired = signal(false);
        let selected_label: Signal<String> = signal(String::new());
        let (mut cx, mut layout, mut router, root) = setup(move || {
            command_menu()
                .item("Attack", move || fired.set(true))
                .submenu("Ability", |m| m.item("Fire", || {}))
                .select(move |label| selected_label.set(label.to_owned()))
        });

        // Descend into Ability, then fire its one leaf.
        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (Key::ArrowDown, PhysicalKey::ArrowDown),
        );
        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (Key::ArrowRight, PhysicalKey::ArrowRight),
        );
        assert_eq!(
            panel_count(&cx, root),
            2,
            "right-arrow descends into Ability"
        );

        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (Key::Enter, PhysicalKey::Enter),
        );
        assert_eq!(
            selected_label.get(),
            "Fire",
            "select observer receives the fired leaf's label"
        );
        assert_eq!(
            panel_count(&cx, root),
            1,
            "firing a nested leaf closes back to the root panel"
        );
        assert!(
            !fired.get(),
            "Attack's own action must not fire for a Fire selection"
        );
    });
}

// ---- goldens ------------------------------------------------------------------

const DELTA_PCT: f32 = 0.5;

fn check(stem: &str, img: Option<image::RgbaImage>) {
    let Some(img) = img else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    assert_png_matches_golden(&img, &golden_path(stem), DELTA_PCT);
}

/// Pre-navigate a freshly-added command menu with `keys`, using a
/// throwaway `LayoutEngine` + `EventRouter` so the widget's own
/// reactive navigation stack reflects the sequence before the golden
/// harness's own layout/emit pass captures it.
fn navigate(cx: &mut Cx, root: NodeId, keys: &[(Key, PhysicalKey)]) {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(VIEWPORT.0, VIEWPORT.1))
        .expect("layout");
    let mut router = EventRouter::new();
    router.set_focus(cx, Some(root));
    for (key, physical) in keys {
        router.route(cx, &layout, key_down(*key, *physical));
    }
}

#[test]
fn command_menu_open() {
    let img = render_widget_to_png(
        |cx| cx.add(sample_menu(|_| {})()),
        InteractionMode::Default,
        [432, 128],
        (1.0, 1.0),
    );
    check("command_menu_open", img);
}

#[test]
fn command_menu_nested() {
    let img = render_widget_to_png(
        |cx| {
            let root = cx.add(sample_menu(|_| {})());
            navigate(
                cx,
                root,
                &[
                    (Key::ArrowDown, PhysicalKey::ArrowDown),
                    (Key::Enter, PhysicalKey::Enter),
                ],
            );
            root
        },
        InteractionMode::Default,
        [432, 128],
        (1.0, 1.0),
    );
    check("command_menu_nested", img);
}

#[test]
fn command_menu_disabled_item() {
    let img = render_widget_to_png(
        |cx| {
            let root = cx.add(sample_menu(|_| {})());
            // Attack -> Ability -> (skip disabled Item) -> Wait: proves the
            // dimmed "Item" row renders distinctly while navigation skips it.
            navigate(
                cx,
                root,
                &[
                    (Key::ArrowDown, PhysicalKey::ArrowDown),
                    (Key::ArrowDown, PhysicalKey::ArrowDown),
                ],
            );
            root
        },
        InteractionMode::Default,
        [432, 128],
        (1.0, 1.0),
    );
    check("command_menu_disabled_item", img);
}
