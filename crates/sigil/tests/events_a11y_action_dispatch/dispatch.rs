use super::*;

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
