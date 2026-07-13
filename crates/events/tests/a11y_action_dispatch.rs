//! W5-A11Y (DR-050 §4) — AT-initiated `accesskit::ActionRequest` dispatch.
//!
//! Mirrors exactly what `crates/app/src/launch/event_wiring.rs`'s
//! `drain_a11y_actions` does (a validated match on `request.action`),
//! since that function is private to the `app` crate. This proves the
//! primitives it calls — `EventRouter::{is_focus_target, set_focus,
//! is_click_target}` and `events::invoke_node` — actually produce the
//! sigil-side effect a screen reader's Focus/Invoke gesture expects
//! (focus moves, a button's click-activation handler fires through the
//! SAME hover+press gate a real pointer click uses), AND that AT
//! requests are held to the same interactivity contract real
//! pointer/keyboard input already enforces (RC-focus adversarial
//! finding 3: a platform request must not focus behind a modal or
//! activate a hidden/non-focusable control).

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "support/mock_shell.rs"]
mod mock_shell;
#[path = "support/routing_helpers.rs"]
mod routing_helpers;

use std::cell::Cell;
use std::rc::Rc;

use accesskit::{Action, ActionRequest, TreeId};
use allocation_counter as _;
use criterion as _;
use events::prelude::*;
use kernel::FocusScopeKind;
use kernel::prelude::*;
use layout::prelude::*;
use reactive::{Runtime, signal};
use smallvec as _;
use style::{DisplayKind, NodeStyleExt, StyleAtom};

use routing_helpers::{compute_layout, pointer_move, route_event};

/// Route one synthetic `ActionRequest` exactly as `drain_a11y_actions`
/// does — the production validated-match this test exists to verify.
fn dispatch(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, request: &ActionRequest) {
    let target = NodeId::from_accesskit(request.target_node);
    match request.action {
        Action::Focus if router.is_focus_target(cx, layout, target) => {
            router.set_focus(cx, Some(target));
        }
        Action::Click if router.is_click_target(cx, layout, target) => {
            events::invoke_node(cx, layout, target);
        }
        _ => {}
    }
}

fn scope_marker(kind: FocusScopeKind) -> impl FnOnce(&mut kernel::Node) {
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

    // Mirrors `sigil::interaction::InteractionState`'s hover/press gate
    // (`crates/sigil/src/interaction.rs`): `release_pointer` only fires
    // when BOTH `hovered` and `pressed` are true at the moment Up
    // arrives. A bare synthetic Down+Up (no Enter) would leave `hovered`
    // false and silently no-op here, exactly as it would in production.
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
    // DR-050 §4 / Wave-5 design brief item 7: per-widget state-signal
    // actions (Increment/Decrement/SetValue/...) are out of scope for
    // this slice — assert the dispatcher does not panic or move focus
    // on an action it doesn't yet handle.
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

// ---- RC-focus adversarial finding 3: AT actions must obey the same
// interactivity gates real pointer/keyboard input does. ------------------

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

/// Builds root -> [focusable/clickable `outside` sibling, active modal
/// scope with its own focusable inside]. `outside` carries a
/// `pointer_up` handler that flips `activated` so click-suppression is
/// observable the same way `click_on_a_hidden_node_is_a_no_op` checks it.
/// Drives one real event first so `sync_scopes` registers the modal.
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

// ---- RC-focus adversarial finding (round 2, HIGH): validation must use
// THIS frame's fresh layout, not a stale pre-recompute snapshot. --------

fn hidden_atoms(show: reactive::Signal<bool>) -> &'static [StyleAtom] {
    if show.get() {
        &[]
    } else {
        &[StyleAtom::Display(DisplayKind::None)]
    }
}

/// A root with one focusable CHILD whose `Display` is bound to `show` —
/// mirrors `sigil::widgets::modal`'s own show/hide wiring (proven by
/// a4972ca's `dismissed_modal_releases_focus_trap`), and matters
/// mechanically: `Display::None` excludes a node from ITS PARENT's
/// layout, so the hidden node must be a child, not the compute root
/// itself (a `Display::None` compute root still gets a result — there is
/// no parent flow to exclude it from).
fn hideable_child_tree(cx: &mut Cx, show: reactive::Signal<bool>) -> (NodeId, NodeId) {
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
    // "queue an action, hide the target in the same frame via a signal,
    // run the frame — action must be rejected." Mirrors production:
    // drain_a11y_actions now runs AFTER recompute_layout, so the
    // validator sees the FRESH (post-hide) layout, not last frame's.
    Runtime::scope("a11y::hide_then_validate", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let (root, child) = hideable_child_tree(&mut cx, show);
        let mut layout = compute_layout(&mut cx, root, 200.0, 200.0);
        let mut router = EventRouter::new();
        assert!(
            router.is_focus_target(&cx, &layout, child),
            "sanity: the visible node is a valid focus target before hiding"
        );

        // Hide via signal THIS frame (mirrors an Effect-phase mutation
        // that fires before the drain), then recompute — the "run the
        // frame" step.
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
fn stale_pre_recompute_layout_would_have_wrongly_allowed_the_now_hidden_target() {
    // Documents WHY the ordering fix matters: the SAME validator, queried
    // against a layout snapshot that has NOT been recomputed since the
    // hide (what the pre-fix `drain_a11y_actions` — called from inside
    // `drain_events`, before `recompute_layout` — would have seen), still
    // reports the node as a valid target. This is the exact bug the
    // ordering fix in `frame_once` corrects.
    Runtime::scope("a11y::stale_layout_control", || {
        let mut cx = Cx::new_root();
        let show = signal(true);
        let (root, child) = hideable_child_tree(&mut cx, show);
        let layout = compute_layout(&mut cx, root, 200.0, 200.0);
        let mut router = EventRouter::new();

        show.set(false); // hidden this frame, but layout NOT recomputed yet

        assert!(
            router.is_focus_target(&cx, &layout, child),
            "control case: validating against a not-yet-recomputed layout \
             still sees the just-hidden target as valid — this is exactly \
             what moving the drain after recompute_layout fixes"
        );
    });
}

// ---- RC-focus adversarial finding (round 2, MEDIUM): a node scrolled
// outside the viewport must not be a valid AT click target. -------------

#[test]
fn click_on_a_node_scrolled_outside_the_viewport_is_a_no_op() {
    Runtime::scope("a11y::scrolled_out_of_view", || {
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

        // Scroll the node far outside the 200x200 viewport (mirrors a
        // list item scrolled above/below the visible area) and recompute.
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
