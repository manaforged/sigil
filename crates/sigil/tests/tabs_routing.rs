#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::similar_names,
    clippy::float_cmp,
    clippy::suboptimal_flops,
    clippy::too_many_lines,
    clippy::doc_markdown,
    clippy::shadow_unrelated
)]

//! Phase 2.3 — integration tests for the tabs widget.
//!
//! Covers the three features the wave shipped:
//! - content routing via [`TabsBuilder::content_for`] — only the active
//!   tab's panel paints; siblings collapse to a zero-sized layout box.
//! - close affordance via [`TabsBuilder::closeable`] +
//!   [`TabsBuilder::close`] — each tab gains a `Role::Button` child
//!   labelled "Close tab" that fires the handler when activated.
//! - drag-reorder via [`TabsBuilder::reorderable`] +
//!   [`TabsBuilder::order`] — pointer-drag swaps entries in the bound
//!   order signal; the non-reorderable form keeps the signal untouched.

use std::cell::RefCell;
use std::rc::Rc;

use sigil::prelude::*;
use sigil_hal::{MouseButton, MouseButtons, Point, PointerEventInit};

const VIEWPORT: Size = Size::fixed(640.0, 480.0);

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout.compute(cx, root, VIEWPORT).expect("layout computes");
    layout
}

const fn pointer_event(
    x: f32,
    y: f32,
    phase: PointerPhase,
    button: Option<MouseButton>,
    buttons: MouseButtons,
) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button,
        buttons,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    pointer_event(
        x,
        y,
        PointerPhase::Down,
        Some(MouseButton::Left),
        MouseButtons::LEFT,
    )
}

const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    pointer_event(x, y, PointerPhase::Move, None, MouseButtons::LEFT)
}

const fn pointer_up(x: f32, y: f32) -> ShellEvent {
    pointer_event(
        x,
        y,
        PointerPhase::Up,
        Some(MouseButton::Left),
        MouseButtons::empty(),
    )
}

fn find_by_role_label(cx: &Cx, root: NodeId, role: Role, label: &str) -> Vec<NodeId> {
    let mut hits = Vec::new();
    cx.visit_preorder(root, |id, node| {
        if node.a11y.role == role && matches_static_label(&node.a11y.name, label) {
            hits.push(id);
        }
    });
    hits
}

fn find_all_by_role(cx: &Cx, root: NodeId, role: Role) -> Vec<NodeId> {
    let mut hits = Vec::new();
    cx.visit_preorder(root, |id, node| {
        if node.a11y.role == role {
            hits.push(id);
        }
    });
    hits
}

fn matches_static_label(name: &AccessibleName, expected: &str) -> bool {
    matches!(name, AccessibleName::Static(value) if value == expected)
}

fn ancestor_role(cx: &Cx, id: NodeId, role: Role) -> Option<NodeId> {
    let mut current = cx.parent(id);
    while let Some(parent) = current {
        if let Some(node) = cx.get(parent)
            && node.a11y.role == role
        {
            return Some(parent);
        }
        current = cx.parent(parent);
    }
    None
}

/// Active panel must materialise with a non-zero layout box; inactive
/// panels collapse — taffy returns no measured rect for the
/// `Display::None` subtree the builder paints inactive tabs with, so
/// `layout.result()` is `None` for the user-supplied content child.
#[test]
fn routing_only_materialises_active_content() {
    sigil::reactive::Runtime::scope("sigil-tabs-routing-content", || {
        let active = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(
            tabs(active, vec!["One".into(), "Two".into(), "Three".into()])
                .content_for(
                    0,
                    container()
                        .label("panel-0")
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0)),
                )
                .content_for(
                    1,
                    container()
                        .label("panel-1")
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0)),
                )
                .content_for(
                    2,
                    container()
                        .label("panel-2")
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0)),
                ),
        );
        let layout = compute(&mut cx, root);

        // All three panel nodes are present in the kernel tree — content
        // routing is implemented via `Display::None` on the wrapper, not
        // by removing nodes.
        let panel0 = find_by_role_label(&cx, root, Role::GenericContainer, "panel-0");
        let panel1 = find_by_role_label(&cx, root, Role::GenericContainer, "panel-1");
        let panel2 = find_by_role_label(&cx, root, Role::GenericContainer, "panel-2");
        assert_eq!(panel0.len(), 1, "panel 0 must exist in the kernel tree");
        assert_eq!(panel1.len(), 1, "panel 1 must exist in the kernel tree");
        assert_eq!(panel2.len(), 1, "panel 2 must exist in the kernel tree");

        // Only the active tab's panel has a measured layout box. The
        // inactive panels live under a `Display::None` wrapper so taffy
        // never reaches them and `layout.result()` returns `None`.
        let h0 = layout
            .result(panel0[0])
            .map_or(0.0, |result| result.rect().height);
        let h1 = layout
            .result(panel1[0])
            .map_or(0.0, |result| result.rect().height);
        let h2 = layout
            .result(panel2[0])
            .map_or(0.0, |result| result.rect().height);
        assert!(h0 > 0.0, "active panel must paint (height = {h0})");
        assert!(h1 == 0.0, "inactive panel 1 must collapse (height = {h1})");
        assert!(h2 == 0.0, "inactive panel 2 must collapse (height = {h2})");

        // Switch to tab 2 and re-compute — previously-active panel 0
        // collapses, panel 2 takes its place.
        active.set(2);
        let layout = compute(&mut cx, root);
        let h0 = layout
            .result(panel0[0])
            .map_or(0.0, |result| result.rect().height);
        let h1 = layout
            .result(panel1[0])
            .map_or(0.0, |result| result.rect().height);
        let h2 = layout
            .result(panel2[0])
            .map_or(0.0, |result| result.rect().height);
        assert!(
            h0 == 0.0,
            "panel 0 must collapse after active=2 (height = {h0})"
        );
        assert!(h1 == 0.0, "panel 1 still collapsed (height = {h1})");
        assert!(
            h2 > 0.0,
            "panel 2 must paint after active=2 (height = {h2})"
        );
    });
}

/// The closeable opt-in must install a `Role::Button` child per tab,
/// labelled "Close tab", with its own pointer-up handler that routes
/// back through the registered close callback. We assert the kernel +
/// layout wiring is in place; the pointer-driven activation path itself
/// is exercised in a follow-up once the close button's hover-tracking
/// gap (see test below) lands.
#[test]
fn close_button_wired_into_each_tab() {
    sigil::reactive::Runtime::scope("sigil-tabs-routing-close-wiring", || {
        let active = signal(0_usize);
        let received = Rc::new(RefCell::new(Vec::<usize>::new()));
        let recorder = {
            let received = Rc::clone(&received);
            move |index: usize| received.borrow_mut().push(index)
        };
        let mut cx = Cx::new_root();
        let root = cx.add(
            tabs(active, vec!["A".into(), "B".into(), "C".into()])
                .closeable(true)
                .close(recorder),
        );
        let layout = compute(&mut cx, root);

        // One Role::Button per tab, all labelled "Close tab", each one
        // mounted under its owning Role::Tab.
        let close_buttons = find_by_role_label(&cx, root, Role::Button, "Close tab");
        assert_eq!(close_buttons.len(), 3, "one close button per tab");
        let tabs_in_order = find_all_by_role(&cx, root, Role::Tab);
        assert_eq!(tabs_in_order.len(), 3, "three tabs");
        for (visual_index, close_id) in close_buttons.iter().copied().enumerate() {
            let owning_tab = ancestor_role(&cx, close_id, Role::Tab).expect("owning tab");
            assert_eq!(
                owning_tab, tabs_in_order[visual_index],
                "close button {visual_index} must be mounted under tab {visual_index}"
            );
            // Layout must reach the close affordance — without a real
            // measured rect, downstream renderers + hit-testing fail
            // silently.
            let close_rect = layout.result(close_id).expect("close button layout").rect();
            assert!(
                close_rect.width > 0.0 && close_rect.height > 0.0,
                "close button {visual_index} must have a non-zero rect ({close_rect:?})"
            );
        }
        // No close has fired yet; the handler vector starts empty.
        assert!(
            received.borrow().is_empty(),
            "close handler must not fire before user activation"
        );
    });
}

/// Without `.reorderable(true)`, pointer drags must leave the bound
/// `order` signal alone. This is the only drag scenario that runs
/// end-to-end through `EventRouter` today — the reorder happy path is
/// gated on a drag-handler placement gap (see the ignored test below).
#[test]
fn non_reorderable_ignores_drag() {
    sigil::reactive::Runtime::scope("sigil-tabs-routing-non-reorderable", || {
        let active = signal(0_usize);
        let order = signal(vec![0_usize, 1, 2]);
        let mut cx = Cx::new_root();
        let root =
            cx.add(tabs(active, vec!["Alpha".into(), "Beta".into(), "Gamma".into()]).order(order));
        let layout = compute(&mut cx, root);

        let tab_nodes = find_all_by_role(&cx, root, Role::Tab);
        assert_eq!(tab_nodes.len(), 3);
        let tab0 = layout.result(tab_nodes[0]).expect("tab0 layout").rect();
        let tab1 = layout.result(tab_nodes[1]).expect("tab1 layout").rect();
        let start_x = tab0.origin.x + tab0.width * 0.5;
        let start_y = tab0.origin.y + tab0.height * 0.5;
        let target_x = tab1.origin.x + tab1.width * 0.5 + 4.0;

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(start_x, start_y));
        router.route(&mut cx, &layout, pointer_move(target_x, start_y));
        router.route(&mut cx, &layout, pointer_up(target_x, start_y));

        assert_eq!(
            order.get(),
            vec![0_usize, 1, 2],
            "non-reorderable tabs must leave the order signal untouched"
        );
    });
}

/// Reorder happy path — see followups doc for fix recipes.
#[test]
fn reorder_swaps_tab_indices_on_drag() {
    sigil::reactive::Runtime::scope("sigil-tabs-routing-reorder", || {
        let active = signal(0_usize);
        let order = signal(vec![0_usize, 1, 2]);
        let mut cx = Cx::new_root();
        let root = cx.add(
            tabs(active, vec!["Alpha".into(), "Beta".into(), "Gamma".into()])
                .reorderable(true)
                .order(order),
        );
        let layout = compute(&mut cx, root);

        let tab_nodes = find_all_by_role(&cx, root, Role::Tab);
        assert_eq!(tab_nodes.len(), 3);
        let tab0 = layout.result(tab_nodes[0]).expect("tab0 layout").rect();
        let tab1 = layout.result(tab_nodes[1]).expect("tab1 layout").rect();
        let start_x = tab0.origin.x + tab0.width * 0.5;
        let start_y = tab0.origin.y + tab0.height * 0.5;
        let target_x = tab1.origin.x + tab1.width * 0.5 + 4.0;

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(start_x, start_y));
        router.route(&mut cx, &layout, pointer_move(target_x, start_y));
        router.route(&mut cx, &layout, pointer_up(target_x, start_y));

        assert_eq!(
            order.get(),
            vec![1_usize, 0, 2],
            "drag past tab 1 midpoint must swap visual positions 0 and 1"
        );
        assert_eq!(
            active.get(),
            0,
            "active tab tracks the original index, not the visual slot"
        );
    });
}

/// Close-button activation happy path — see followups doc for fix recipes.
#[test]
fn close_button_fires_callback_with_index() {
    sigil::reactive::Runtime::scope("sigil-tabs-routing-close", || {
        let active = signal(0_usize);
        let received = Rc::new(RefCell::new(Vec::<usize>::new()));
        let recorder = {
            let received = Rc::clone(&received);
            move |index: usize| received.borrow_mut().push(index)
        };
        let mut cx = Cx::new_root();
        let root = cx.add(
            tabs(active, vec!["A".into(), "B".into(), "C".into()])
                .closeable(true)
                .close(recorder),
        );
        let layout = compute(&mut cx, root);

        let close_buttons = find_by_role_label(&cx, root, Role::Button, "Close tab");
        let close_for_tab_1 = close_buttons[1];
        let close_rect = layout.result(close_for_tab_1).expect("close layout").rect();
        let cx_centre = close_rect.origin.x + close_rect.width * 0.5;
        let cy_centre = close_rect.origin.y + close_rect.height * 0.5;

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(cx_centre, cy_centre));
        router.route(&mut cx, &layout, pointer_up(cx_centre, cy_centre));

        assert_eq!(received.borrow().as_slice(), &[1]);
        assert_eq!(active.get(), 0);
    });
}
