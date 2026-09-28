use super::*;

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
