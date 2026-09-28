#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::doc_markdown, clippy::too_many_lines, clippy::print_stdout)]

#[path = "events_support/input.rs"]
mod input;

use sigil::hal::PhysicalKey;
use sigil::prelude::*;

use input::key_down;
use sigil_kittest::stub_text_measure;
use sigil_kittest::{
    InteractionMode, assert_rendered_matches_golden, golden_path, render_widget_to_png_stub,
};

const VIEWPORT: (f32, f32) = (420.0, 260.0);

fn sample_menu(select: impl Fn(&str) + 'static) -> impl FnOnce() -> CommandMenuBuilder + 'static {
    move || {
        command_menu()
            .width(180.0)
            .item("Attack", || {})
            .submenu("Ability", |m| {
                m.item("Fire", || {}).item("Cure", || {}).disabled("Ultima")
            })
            .disabled("Item")
            .item("Wait", || {})
            .select(select)
    }
}

fn setup(build: impl FnOnce() -> CommandMenuBuilder) -> (Cx, LayoutEngine, EventRouter, NodeId) {
    let mut cx = Cx::new_root();
    let root = cx.add(build());
    let mut layout = LayoutEngine::with_text_measure(stub_text_measure());
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

const DELTA_PCT: f32 = 0.5;

fn navigate(cx: &mut Cx, root: NodeId, keys: &[(Key, PhysicalKey)]) {
    let mut layout = LayoutEngine::with_text_measure(stub_text_measure());
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
    let img = render_widget_to_png_stub(
        |cx| cx.add(sample_menu(|_| {})()),
        InteractionMode::Default,
        [432, 128],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("command_menu_open"), DELTA_PCT);
}

#[test]
fn command_menu_nested() {
    let img = render_widget_to_png_stub(
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
    assert_rendered_matches_golden(img, &golden_path!("command_menu_nested"), DELTA_PCT);
}

#[test]
fn command_menu_disabled_item() {
    let img = render_widget_to_png_stub(
        |cx| {
            let root = cx.add(sample_menu(|_| {})());
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
    assert_rendered_matches_golden(img, &golden_path!("command_menu_disabled_item"), DELTA_PCT);
}
