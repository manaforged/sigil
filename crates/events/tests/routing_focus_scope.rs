#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

#[path = "support/mock_shell.rs"]
mod mock_shell;
#[path = "support/routing_helpers.rs"]
mod routing_helpers;

use sigil_events::prelude::*;
use sigil_kernel::FocusScopeKind;
use sigil_kernel::prelude::*;
use sigil_layout::prelude::*;

use routing_helpers::{compute_layout, route_event, tab};

fn scope_marker(kind: FocusScopeKind) -> impl FnOnce(&mut sigil_kernel::Node) {
    move |node| {
        node.a11y.focus_scope = Some(kind);
    }
}

fn focusable_in(parent_size: f32) -> NodeBuilder {
    container()
        .focusable(true)
        .layout(LayoutStyle::block().fixed_size(parent_size, parent_size))
}

fn assert_live_focus(cx: &Cx, router: &EventRouter) -> NodeId {
    let focus = router.focus().unwrap();
    assert!(cx.get(focus).is_some(), "focus must point at a live node");
    focus
}

#[test]
fn three_modal_stack_traps_then_restores_on_pop() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 200.0))
            // Background focusable.
            .child(focusable_in(10.0))
            // Modal A.
            .child(
                container()
                    .mutate(scope_marker(FocusScopeKind::Modal))
                    .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                    .child(focusable_in(10.0))
                    // Modal B (nested).
                    .child(
                        container()
                            .mutate(scope_marker(FocusScopeKind::Modal))
                            .layout(LayoutStyle::block().fixed_size(60.0, 60.0))
                            .child(focusable_in(10.0))
                            // Modal C (nested deepest).
                            .child(
                                container()
                                    .mutate(scope_marker(FocusScopeKind::Modal))
                                    .layout(LayoutStyle::block().fixed_size(30.0, 30.0))
                                    .child(focusable_in(10.0)),
                            ),
                    ),
            ),
    );
    let background = cx.nth_child(root, 0).unwrap();
    let modal_a = cx.nth_child(root, 1).unwrap();
    let inside_a = cx.nth_child(modal_a, 0).unwrap();
    let modal_b = cx.nth_child(modal_a, 1).unwrap();
    let inside_b = cx.nth_child(modal_b, 0).unwrap();
    let modal_c = cx.nth_child(modal_b, 1).unwrap();
    let inside_c = cx.nth_child(modal_c, 0).unwrap();

    let layout = compute_layout(&mut cx, root, 200.0, 200.0);
    let mut router = EventRouter::new();

    // Tabbing should land inside the deepest scope (modal C).
    route_event(&mut router, &mut cx, &layout, tab(false));
    assert_eq!(router.focus(), Some(inside_c));

    // Wrap inside modal C: only one focusable in there, wraps to self.
    route_event(&mut router, &mut cx, &layout, tab(false));
    assert_eq!(router.focus(), Some(inside_c));

    // Close modal C: remove it from the tree, then route any event
    // to trigger sync_scopes (we use a Tab as a no-op trigger).
    assert!(cx.remove(modal_c));
    route_event(&mut router, &mut cx, &layout, tab(false));
    assert_eq!(assert_live_focus(&cx, &router), inside_b);

    // Close modal B mid-stack.
    assert!(cx.remove(modal_b));
    route_event(&mut router, &mut cx, &layout, tab(false));
    assert_eq!(assert_live_focus(&cx, &router), inside_a);

    // Close modal A: scope stack empties; document-level tab takes over.
    assert!(cx.remove(modal_a));
    route_event(&mut router, &mut cx, &layout, tab(false));
    assert_eq!(assert_live_focus(&cx, &router), background);
}

#[test]
fn focus_repair_handles_replacement_and_new_modal_scope() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(200.0, 200.0))
            .child(focusable_in(10.0)),
    );
    let old = cx.nth_child(root, 0).unwrap();
    let mut layout = compute_layout(&mut cx, root, 200.0, 200.0);
    let mut router = EventRouter::new();
    router.set_focus(&mut cx, Some(old));
    router.focus_if_needed(&mut cx, &layout);
    assert_eq!(router.focus(), Some(old));

    assert!(cx.remove(old));
    let replacement = cx.add_child(root, focusable_in(10.0));
    layout
        .compute(&mut cx, root, Size::fixed(200.0, 200.0))
        .unwrap();
    router.focus_if_needed(&mut cx, &layout);
    assert_eq!(router.focus(), Some(replacement));

    let modal = cx.add_child(
        root,
        container()
            .mutate(scope_marker(FocusScopeKind::Modal))
            .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
            .child(focusable_in(10.0)),
    );
    let inside = cx.nth_child(modal, 0).unwrap();
    layout
        .compute(&mut cx, root, Size::fixed(200.0, 200.0))
        .unwrap();
    router.focus_if_needed(&mut cx, &layout);
    assert_eq!(router.focus(), Some(inside));
}
