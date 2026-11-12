#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::doc_markdown, clippy::too_many_lines, clippy::print_stdout)]

use sigil::events::Event;
use sigil::events::EventRouter;
use sigil::events::PhysicalKey;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::widgets::CommandMenuBuilder;

use sigil_kittest::{Golden, Harness, InteractionMode, stub_text_measure};

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
            .submit(select)
    }
}

fn setup(build: impl FnOnce() -> CommandMenuBuilder) -> (Cx, LayoutEngine, EventRouter, NodeId) {
    let mut cx = Cx::new_root();
    let root = cx.add(build());
    let mut layout = LayoutEngine::new(stub_text_measure());
    layout
        .compute(&mut cx, root, Constraint::fixed(VIEWPORT.0, VIEWPORT.1))
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
    key: (NamedKey, PhysicalKey),
) {
    router.route(cx, layout, Event::key_down(key.0).physical(key.1));
    layout
        .compute(cx, root, Constraint::fixed(VIEWPORT.0, VIEWPORT.1))
        .expect("layout after key");
}

fn panel_count(cx: &Cx, root: NodeId) -> usize {
    cx.get(root).expect("command menu root").children().len()
}

#[test]
fn nested_submenu_opens_and_esc_ascends() {
    Owner::scope("cmd-menu-nested-esc", || {
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
            (NamedKey::ArrowDown, PhysicalKey::ArrowDown),
        );
        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (NamedKey::Enter, PhysicalKey::Enter),
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
            (NamedKey::Escape, PhysicalKey::Escape),
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
    Owner::scope("cmd-menu-disabled-skip", || {
        let selected: Signal<Vec<String>> = signal(Vec::new());
        let (mut cx, mut layout, mut router, root) = setup(move || {
            command_menu()
                .item("A", || {})
                .disabled("B")
                .item("C", || {})
                .submit(move |label| selected.update(|v| v.push(label.to_owned())))
        });

        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (NamedKey::ArrowDown, PhysicalKey::ArrowDown),
        );
        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (NamedKey::Enter, PhysicalKey::Enter),
        );

        assert_eq!(selected.get(), vec!["C".to_owned()]);
    });
}

#[test]
fn cancel_fires_on_root_esc() {
    Owner::scope("cmd-menu-cancel", || {
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
            (NamedKey::Escape, PhysicalKey::Escape),
        );
        assert!(cancelled.get(), "esc at the root level must fire cancel");
    });
}

#[test]
fn leaf_select_fires_observer_and_closes() {
    Owner::scope("cmd-menu-select-closes", || {
        let fired = signal(false);
        let selected_label: Signal<String> = signal(String::new());
        let (mut cx, mut layout, mut router, root) = setup(move || {
            command_menu()
                .item("Attack", move || fired.set(true))
                .submenu("Ability", |m| m.item("Fire", || {}))
                .submit(move |label| selected_label.set(label.to_owned()))
        });

        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (NamedKey::ArrowDown, PhysicalKey::ArrowDown),
        );
        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (NamedKey::Enter, PhysicalKey::Enter),
        );
        assert_eq!(panel_count(&cx, root), 2, "enter descends into Ability");

        press(
            &mut cx,
            &mut layout,
            &mut router,
            root,
            (NamedKey::Enter, PhysicalKey::Enter),
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

fn navigate(cx: &mut Cx, root: NodeId, keys: &[(NamedKey, PhysicalKey)]) {
    let mut layout = LayoutEngine::new(stub_text_measure());
    layout
        .compute(cx, root, Constraint::fixed(VIEWPORT.0, VIEWPORT.1))
        .expect("layout");
    let mut router = EventRouter::new();
    router.set_focus(cx, Some(query_field(cx, root)));
    for (key, physical) in keys {
        router.route(cx, &layout, Event::key_down(*key).physical(*physical));
    }
}

fn query_field(cx: &Cx, root: NodeId) -> NodeId {
    let mut field = None;
    cx.visit_preorder(root, |id, node| {
        if field.is_none() && node.a11y.role == sigil::kernel::Role::TextInput {
            field = Some(id);
        }
    });
    field.expect("the command menu has a query field")
}

#[test]
fn command_menu_open() {
    let img = Harness::render(|| sample_menu(|_| {})())
        .stub()
        .mode(InteractionMode::Default)
        .size([432, 176])
        .hit((1.0, 1.0))
        .image();
    Golden::new("command_menu_open")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn command_menu_nested() {
    let img = Harness::render(sigil_kittest::WithCx(|cx: &mut sigil::kernel::Cx| {
        let root = cx.add(sample_menu(|_| {})());
        navigate(
            cx,
            root,
            &[
                (NamedKey::ArrowDown, PhysicalKey::ArrowDown),
                (NamedKey::Enter, PhysicalKey::Enter),
            ],
        );
        root
    }))
    .stub()
    .mode(InteractionMode::Default)
    .size([432, 176])
    .hit((1.0, 1.0))
    .image();
    Golden::new("command_menu_nested")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn command_menu_disabled_item() {
    let img = Harness::render(sigil_kittest::WithCx(|cx: &mut sigil::kernel::Cx| {
        let root = cx.add(sample_menu(|_| {})());
        navigate(
            cx,
            root,
            &[
                (NamedKey::ArrowDown, PhysicalKey::ArrowDown),
                (NamedKey::ArrowDown, PhysicalKey::ArrowDown),
            ],
        );
        root
    }))
    .stub()
    .mode(InteractionMode::Default)
    .size([432, 176])
    .hit((1.0, 1.0))
    .image();
    Golden::new("command_menu_disabled_item")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn clicking_a_row_of_a_filtered_list_runs_that_row() {
    Owner::scope("cmd-menu-filtered-click", filtered_click);
}

fn filtered_click() {
    use sigil::events::{Key, KeyPhase, Modifiers};

    let chosen: Signal<String> = signal(String::new());
    let mut harness = sigil_kittest::Harness::new(move || {
        container().focusable(true).child(
            command_menu()
                .item("Attack", || {})
                .item("Wait", || {})
                .item("Run", || {})
                .submit(move |label| chosen.set(label.to_owned())),
        )
    })
    .pixels(420, 260);
    harness.settle();
    harness
        .send([Event::key(Key::character("R"), KeyPhase::Down)
            .modifiers(Modifiers::empty())
            .text(Some("R".into()))])
        .settle();
    let shown = harness.text();
    assert!(!shown.contains("Attack"), "R filters the list: {shown}");
    let row = harness
        .accessibility_tree()
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some("Run"))
        .find_map(|(_, node)| node.bounds())
        .expect("the filtered list shows Run");
    let at = sigil::geometry::Point::new(
        row.x0.midpoint(row.x1) as f32,
        row.y0.midpoint(row.y1) as f32,
    );
    harness
        .send([Event::pointer_down(at), Event::pointer_up(at)])
        .settle();
    assert_eq!(chosen.get(), "Run");
}

fn bottoms(harness: &mut sigil_kittest::Harness, label: &str) -> (f64, f64) {
    let tree = harness.accessibility_tree();
    let bottom = |pick: &dyn Fn(&accesskit::Node) -> bool| {
        tree.nodes
            .iter()
            .filter(|(_, node)| pick(node))
            .find_map(|(_, node)| node.bounds())
            .map_or(f64::NAN, |rect| rect.y1)
    };
    (
        bottom(&|node| node.role() == accesskit::Role::Menu),
        bottom(&|node| node.label() == Some(label)),
    )
}

#[test]
fn arrowing_past_the_last_visible_row_scrolls_it_into_view() {
    Owner::scope("cmd-menu-scrolls", || {
        let labels: Vec<String> = (1..=30).map(|n| format!("Action {n}")).collect();
        let mut harness = sigil_kittest::Harness::new(move || {
            labels.iter().fold(command_menu(), |menu, label| {
                menu.item(label.clone(), || {})
            })
        })
        .pixels(420, 600);
        harness.settle();
        let (start_panel, start_row) = bottoms(&mut harness, "Action 30");
        assert!(
            start_row > start_panel,
            "a long list is cut: row {start_row} panel {start_panel}"
        );
        harness
            .send(
                (0..29)
                    .map(|_| Event::key_down(NamedKey::ArrowDown).physical(PhysicalKey::ArrowDown)),
            )
            .settle();
        let (panel, row) = bottoms(&mut harness, "Action 30");
        assert!(
            row <= panel,
            "the cursor row is in view: row {row} panel {panel}"
        );
    });
}
