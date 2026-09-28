use super::*;

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
    route_event(&mut router, cx, &layout, pointer_drag(-10.0, -10.0));
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
