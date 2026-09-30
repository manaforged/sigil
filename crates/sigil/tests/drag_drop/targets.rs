use super::*;
use sigil::events::EventRouter;
use sigil::kernel::Cx;

#[test]
fn target_highlight_is_reactive_and_clears_on_move_away() {
    sigil::reactive::Runtime::scope("drag-drop-highlight", || {
        let (_state, drops, _downs, mut cx, root) = demo();
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let highlight = Theme::default().accent_scale.step(4);
        assert!(
            !rect_colors(&cx, root).contains(&highlight),
            "rest: no highlight",
        );

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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert!(
            rect_colors(&cx, root).contains(&highlight),
            "hovered accepting target paints the highlight slice",
        );

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 500.0, 300.0),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert!(
            !rect_colors(&cx, root).contains(&highlight),
            "moving off the target clears the highlight",
        );

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Up, 500.0, 300.0),
        );
        assert!(drops.borrow().is_empty());
    });
}

#[test]
fn rejecting_target_neither_highlights_nor_accepts() {
    sigil::reactive::Runtime::scope("drag-drop-reject", || {
        let state = DragState::<String>::new();
        let drops = Rc::new(RefCell::new(Vec::new()));
        let downs = Rc::new(RefCell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(demo_tree(state.clone(), &drops, &downs, |payload| {
            payload != "task-7"
        }));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let highlight = Theme::default().accent_scale.step(4);
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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert!(
            !rect_colors(&cx, root).contains(&highlight),
            "a rejecting target never highlights",
        );
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Up, 250.0, 20.0),
        );
        assert!(drops.borrow().is_empty(), "a rejecting target gets no drop");
        assert!(!state.is_active(), "the session still ends cleanly");
    });
}

#[test]
fn ghost_chip_lives_only_during_the_session() {
    sigil::reactive::Runtime::scope("drag-drop-ghost", || {
        let (_state, _drops, _downs, mut cx, root) = demo();
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert!(
            !glyph_texts(&cx, root).iter().any(|t| t == "moving task-7"),
            "idle: no ghost chip",
        );

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
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert!(
            glyph_texts(&cx, root).iter().any(|t| t == "moving task-7"),
            "active: the ghost chip renders the payload label",
        );

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Up, 250.0, 20.0),
        );
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert!(
            !glyph_texts(&cx, root).iter().any(|t| t == "moving task-7"),
            "after the drop: the ghost chip is gone",
        );
    });
}
