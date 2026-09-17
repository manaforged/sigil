#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "events_support/mock_shell.rs"]
mod mock_shell;
#[path = "events_support/routing_helpers.rs"]
mod routing_helpers;

use std::cell::Cell;
use std::rc::Rc;

use accesskit::{Action, ActionRequest, TreeId};
use sigil::events::prelude::*;
use sigil::kernel::FocusScopeKind;
use sigil::kernel::prelude::*;
use sigil::layout::prelude::*;
use sigil::reactive::{Runtime, signal};
use sigil::style::{DisplayKind, NodeStyleExt, StyleAtom};

use routing_helpers::{compute_layout, pointer_move, route_event};

fn dispatch(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, request: &ActionRequest) {
    let target = NodeId::from_accesskit(request.target_node);
    match request.action {
        Action::Focus if router.is_focus_target(cx, layout, target) => {
            router.set_focus(cx, Some(target));
        }
        Action::Click if router.is_click_target(cx, layout, target) => {
            sigil::events::invoke_node(cx, layout, target);
        }
        _ => {}
    }
}

fn scope_marker(kind: FocusScopeKind) -> impl FnOnce(&mut sigil::kernel::Node) {
    move |node| {
        node.a11y.focus_scope = Some(kind);
    }
}

fn action_request(action: Action, target: NodeId) -> ActionRequest {
    ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node: target.to_accesskit(),
        data: None,
    }
}

#[test]
fn focus_action_moves_router_focus_to_the_target_node() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(80.0, 32.0)),
    );
    let layout = compute_layout(&mut cx, root, 200.0, 200.0);
    let mut router = EventRouter::new();
    assert_eq!(router.focus(), None, "nothing focused before dispatch");

    dispatch(
        &mut router,
        &mut cx,
        &layout,
        &action_request(Action::Focus, root),
    );

    assert_eq!(
        router.focus(),
        Some(root),
        "Action::Focus must move EventRouter's focus to the AT-named node"
    );
}

#[test]
fn click_action_fires_the_same_hover_press_gate_a_real_pointer_click_uses() {
    let mut cx = Cx::new_root();

    let activated = Rc::new(Cell::new(false));
    let hovered = Rc::new(Cell::new(false));
    let pressed = Rc::new(Cell::new(false));

    let enter_hovered = Rc::clone(&hovered);
    let leave_hovered = Rc::clone(&hovered);
    let leave_pressed = Rc::clone(&pressed);
    let down_pressed = Rc::clone(&pressed);
    let up_hovered = Rc::clone(&hovered);
    let up_pressed = Rc::clone(&pressed);
    let up_activated = Rc::clone(&activated);

    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(80.0, 32.0))
            .pointer_enter(move |_| enter_hovered.set(true))
            .pointer_leave(move |_| {
                leave_hovered.set(false);
                leave_pressed.set(false);
            })
            .pointer_down(move |_| down_pressed.set(true))
            .pointer_up(move |_| {
                if up_hovered.get() && up_pressed.get() {
                    up_activated.set(true);
                }
                up_pressed.set(false);
            }),
    );
    let layout = compute_layout(&mut cx, root, 200.0, 200.0);
    let mut router = EventRouter::new();

    dispatch(
        &mut router,
        &mut cx,
        &layout,
        &action_request(Action::Click, root),
    );

    assert!(
        activated.get(),
        "Action::Click must synthesize Enter->Down->Up (not a bare Down+Up) so the \
         same hover+press activation gate a real pointer click uses fires"
    );
    assert!(
        !hovered.get() && !pressed.get(),
        "the trailing synthetic Leave must reset hover/press state, matching what a \
         real pointer click sequence leaves behind"
    );
}

#[test]
fn unmapped_action_variants_are_a_documented_no_op() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .focusable(true)
            .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
    );
    let layout = compute_layout(&mut cx, root, 200.0, 200.0);
    let mut router = EventRouter::new();

    dispatch(
        &mut router,
        &mut cx,
        &layout,
        &action_request(Action::Increment, root),
    );

    assert_eq!(
        router.focus(),
        None,
        "an unmapped action must not move focus as a side effect"
    );
}

#[test]
fn click_on_a_hidden_node_is_a_no_op() {
    let mut cx = Cx::new_root();
    let activated = Rc::new(Cell::new(false));
    let activated_w = Rc::clone(&activated);
    let root = cx.add(
        container()
            .focusable(true)
            .hittable(false)
            .layout(LayoutStyle::block().fixed_size(80.0, 32.0))
            .pointer_up(move |_| activated_w.set(true)),
    );
    let layout = compute_layout(&mut cx, root, 200.0, 200.0);
    let mut router = EventRouter::new();

    dispatch(
        &mut router,
        &mut cx,
        &layout,
        &action_request(Action::Click, root),
    );

    assert!(
        !activated.get(),
        "Action::Click on a hittable=false (hidden/disabled) node must not fire \
         its activation handler — a real click could never reach it either"
    );
}

#[test]
fn focus_on_a_non_focusable_node_is_a_no_op() {
    let mut cx = Cx::new_root();
    let root = cx.add(container().layout(LayoutStyle::block().fixed_size(80.0, 32.0)));
    let layout = compute_layout(&mut cx, root, 200.0, 200.0);
    let mut router = EventRouter::new();

    dispatch(
        &mut router,
        &mut cx,
        &layout,
        &action_request(Action::Focus, root),
    );

    assert_eq!(
        router.focus(),
        None,
        "Action::Focus on a non-focusable node must not move focus — Tab could \
         never land there either"
    );
}

fn scoped_tree_with_outside_sibling(
    cx: &mut Cx,
    activated: &Rc<Cell<bool>>,
) -> (NodeId, EventRouter, LayoutEngine) {
    let activated_w = Rc::clone(activated);
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 200.0))
            .child(
                container()
                    .focusable(true)
                    .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                    .pointer_up(move |_| activated_w.set(true)),
            )
            .child(
                container()
                    .mutate(scope_marker(FocusScopeKind::Modal))
                    .layout(LayoutStyle::block().fixed_size(50.0, 50.0))
                    .child(
                        container()
                            .focusable(true)
                            .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
                    ),
            ),
    );
    let outside = cx.nth_child(root, 0).expect("outside child should exist");
    let layout = compute_layout(cx, root, 200.0, 200.0);
    let mut router = EventRouter::new();
    route_event(&mut router, cx, &layout, pointer_move(-10.0, -10.0));
    (outside, router, layout)
}

#[test]
fn focus_outside_the_active_modal_scope_is_a_no_op() {
    let mut cx = Cx::new_root();
    let (outside, mut router, layout) =
        scoped_tree_with_outside_sibling(&mut cx, &Rc::new(Cell::new(false)));

    dispatch(
        &mut router,
        &mut cx,
        &layout,
        &action_request(Action::Focus, outside),
    );

    assert_eq!(
        router.focus(),
        None,
        "Action::Focus on a node outside the active modal scope must not move \
         focus — Tab is confined to the topmost scope, so AT Focus must be too"
    );
}

#[test]
fn click_outside_the_active_modal_scope_is_a_no_op() {
    let mut cx = Cx::new_root();
    let activated = Rc::new(Cell::new(false));
    let (outside, mut router, layout) = scoped_tree_with_outside_sibling(&mut cx, &activated);

    dispatch(
        &mut router,
        &mut cx,
        &layout,
        &action_request(Action::Click, outside),
    );

    assert!(
        !activated.get(),
        "Action::Click on a node outside the active modal scope must not fire — \
         invoke_node bypasses hit-testing entirely, so nothing else would stop it \
         reaching behind the modal"
    );
}

fn hidden_atoms(show: sigil::reactive::Signal<bool>) -> &'static [StyleAtom] {
    if show.get() {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

fn hideable_child_tree(cx: &mut Cx, show: sigil::reactive::Signal<bool>) -> (NodeId, NodeId) {
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 200.0))
            .child(
                container()
                    .focusable(true)
                    .layout(LayoutStyle::block().fixed_size(80.0, 32.0))
                    .style_reactive(move || hidden_atoms(show)),
            ),
    );
    let child = cx.nth_child(root, 0).expect("child must exist");
    (root, child)
}

#[test]
fn action_rejected_when_target_hidden_via_signal_before_the_validating_recompute() {
    Runtime::scope("sigil_a11y::hide_then_validate", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let (root, child) = hideable_child_tree(&mut cx, show);
        let mut layout = compute_layout(&mut cx, root, 200.0, 200.0);
        let mut router = EventRouter::new();
        assert!(
            router.is_focus_target(&cx, &layout, child),
            "sanity: the visible node is a valid focus target before hiding"
        );

        show.set(false);
        layout
            .compute(&mut cx, root, Size::fixed(200.0, 200.0))
            .expect("recompute after hide must succeed");

        dispatch(
            &mut router,
            &mut cx,
            &layout,
            &action_request(Action::Focus, child),
        );

        assert_eq!(
            router.focus(),
            None,
            "Action::Focus on a node hidden via signal THIS frame, validated \
             against the FRESH post-hide layout, must be rejected"
        );
    });
}

#[test]
fn hidden_target_is_rejected_before_layout_recomputes() {
    Runtime::scope("sigil_a11y::hidden_before_recompute", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let (root, child) = hideable_child_tree(&mut cx, show);
        let layout = compute_layout(&mut cx, root, 200.0, 200.0);
        show.set(false);
        assert!(!EventRouter::new().is_focus_target(&cx, &layout, child));
    });
}

#[test]
fn click_on_a_node_scrolled_outside_the_viewport_is_a_no_op() {
    Runtime::scope("sigil_a11y::scrolled_out_of_view", || {
        let mut cx = Cx::new_root();
        let pos = signal((10.0_f32, 10.0_f32));
        let activated = Rc::new(Cell::new(false));
        let activated_w = Rc::clone(&activated);
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(200.0, 200.0))
                .child(
                    container()
                        .focusable(true)
                        .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                        .position_reactive(pos)
                        .pointer_up(move |_| activated_w.set(true)),
                ),
        );
        let scrolled = cx.nth_child(root, 0).expect("child must exist");
        let mut layout = compute_layout(&mut cx, root, 200.0, 200.0);
        let mut router = EventRouter::new();
        assert!(
            router.is_click_target(&cx, &layout, scrolled),
            "sanity: on-screen node is a valid click target before scrolling"
        );

        pos.set((5000.0, 5000.0));
        layout
            .compute(&mut cx, root, Size::fixed(200.0, 200.0))
            .expect("recompute after scroll must succeed");

        dispatch(
            &mut router,
            &mut cx,
            &layout,
            &action_request(Action::Click, scrolled),
        );

        assert!(
            !activated.get(),
            "Action::Click on a node scrolled outside the viewport must not \
             fire — invoke_node dispatches directly by node id with no \
             position, so nothing else would stop it reaching off-screen \
             content"
        );
    });
}
