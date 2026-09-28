use super::*;

#[test]
fn plain_click_and_right_click_never_start_a_session() {
    sigil::reactive::Runtime::scope("drag-drop-click", || {
        let (state, drops, downs, mut cx, root) = demo();
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Down, 10.0, 10.0),
        );
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Up, 10.0, 10.0),
        );
        assert!(!state.is_active(), "click without movement stays idle");
        assert_eq!(state.payload(), None);
        assert_eq!(*downs.borrow(), 1, "the row's own down handler still fired");

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Down, 10.0, 10.0),
        );
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 12.0, 10.0),
        );
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Up, 12.0, 10.0),
        );
        assert!(!state.is_active(), "sub-threshold jitter stays idle");

        route(&mut router, &mut cx, &layout, right_down(10.0, 10.0));
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 60.0, 30.0),
        );
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Up, 60.0, 30.0),
        );
        assert!(!state.is_active(), "right-drag never starts a session");
        assert!(drops.borrow().is_empty(), "no drops fired");
        assert_eq!(
            *downs.borrow(),
            3,
            "the row's own down handler sees every press — drag arming is separate",
        );
    });
}

#[test]
fn drag_past_threshold_activates_and_survives_leaving_the_source() {
    sigil::reactive::Runtime::scope("drag-drop-threshold", || {
        let (state, drops, _downs, mut cx, root) = demo();
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Down, 10.0, 10.0),
        );
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 12.0, 11.0),
        );
        assert!(!state.is_active(), "2.2 px of travel is below threshold");

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 30.0, 25.0),
        );
        assert!(state.is_active(), "25 px of travel starts the session");
        assert_eq!(state.payload().as_deref(), Some("task-7"));
        assert_eq!(state.position(), (30.0, 25.0));

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 500.0, 300.0),
        );
        assert!(state.is_active());
        assert_eq!(state.position(), (500.0, 300.0));

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Up, 500.0, 300.0),
        );
        assert!(!state.is_active(), "drop outside any target cancels");
        assert_eq!(state.payload(), None);
        assert!(drops.borrow().is_empty(), "cancel delivers nothing");
    });
}

#[test]
fn drop_on_accepting_target_fires_the_payload() {
    sigil::reactive::Runtime::scope("drag-drop-deliver", || {
        let (state, drops, _downs, mut cx, root) = demo();
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Down, 10.0, 10.0),
        );
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 30.0, 25.0),
        );
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 250.0, 20.0),
        );
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Up, 250.0, 20.0),
        );

        assert_eq!(drops.borrow().as_slice(), &["task-7".to_owned()]);
        assert!(!state.is_active(), "session ends on drop");
        assert_eq!(state.payload(), None, "payload clears on drop");
    });
}
