use super::*;
use sigil::events::EventRouter;
use sigil::kernel::Cx;

#[test]
fn target_highlight_is_reactive_and_clears_on_move_away() {
    sigil::reactive::Owner::scope("drag-drop-highlight", || {
        let (_state, drops, _downs, mut cx, root) = demo();
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let highlight = Theme::default().scale(Tone::Primary).step(4);
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
    sigil::reactive::Owner::scope("drag-drop-reject", || {
        let state = DragState::<String>::new();
        let drops = Rc::new(RefCell::new(Vec::new()));
        let downs = Rc::new(RefCell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(demo_tree(state.clone(), &drops, &downs, |payload| {
            payload != "task-7"
        }));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let highlight = Theme::default().scale(Tone::Primary).step(4);
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
    sigil::reactive::Owner::scope("drag-drop-ghost", || {
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

type DropCapture = Rc<RefCell<Option<sigil::events::DropEvent<String>>>>;

struct DropHoverFixture {
    hover: Signal<Option<Point>>,
    dropped: DropCapture,
    cx: Cx,
    root: NodeId,
    _state: DragState<String>,
}

fn drop_hover_fixture() -> DropHoverFixture {
    let state = DragState::<String>::new();
    let hover = signal(None::<Point>);
    let dropped: DropCapture = Rc::new(RefCell::new(None));
    let dropped_w = Rc::clone(&dropped);
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::flex_row().fixed_size(WIDTH, HEIGHT))
            .child(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                    .drag_source(state.clone(), "task-9".to_owned()),
            )
            .child(container().layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0)))
            .child(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                    .drop_hover(state.clone(), hover)
                    .drop_target(
                        state.clone(),
                        |_| true,
                        move |event| *dropped_w.borrow_mut() = Some(event),
                    ),
            ),
    );
    DropHoverFixture {
        hover,
        dropped,
        cx,
        root,
        _state: state,
    }
}

#[test]
fn drop_event_carries_geometry_and_drop_hover_suppresses_default_highlight() {
    sigil::reactive::Owner::scope("drag-drop-event-and-hover", || {
        let DropHoverFixture {
            hover,
            dropped,
            mut cx,
            root,
            _state,
        } = drop_hover_fixture();
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let highlight = Theme::default().scale(Tone::Primary).step(4);
        let mut router = EventRouter::new();

        for (phase, x, y) in [
            (PointerPhase::Down, 10.0, 10.0),
            (PointerPhase::Move, 30.0, 25.0),
            (PointerPhase::Move, 235.0, 28.0),
        ] {
            route(&mut router, &mut cx, &layout, pointer(phase, x, y));
        }
        assert_eq!(hover.peek(), Some(Point::new(35.0, 28.0)));
        assert!(
            !rect_colors(&cx, root).contains(&highlight),
            "drop_hover suppresses the default bg-accent-4 highlight",
        );

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 500.0, 300.0),
        );
        assert_eq!(hover.peek(), None);

        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Move, 235.0, 28.0),
        );
        assert_eq!(hover.peek(), Some(Point::new(35.0, 28.0)));
        route(
            &mut router,
            &mut cx,
            &layout,
            pointer(PointerPhase::Up, 235.0, 28.0),
        );
        assert_eq!(hover.peek(), None);

        let event = dropped.borrow_mut().take().expect("drop event delivered");
        assert_eq!(event.value().as_str(), "task-9");
        assert_eq!(event.position(), Point::new(235.0, 28.0));
        assert_eq!(event.local_position(), Point::new(35.0, 28.0));
        assert_eq!(
            event.bounds(),
            sigil::geometry::Rect::new(Point::new(200.0, 0.0), 100.0, 40.0)
        );
        assert_eq!(event.into_value(), "task-9");
    });
}

#[test]
fn hover_write_can_rebuild_a_nested_drop_target() {
    sigil::reactive::Owner::scope("drag-drop-hover-rebuild", || {
        let state = DragState::<String>::new();
        let cursor = signal(None::<Point>);
        let nested = state.clone();
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(WIDTH, HEIGHT))
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                        .drag_source(state.clone(), "task-7".to_owned())
                        .child(text("task-7")),
                )
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(200.0, 100.0))
                        .drop_target(state.clone(), |_| true, |_| {})
                        .drop_hover(state, cursor)
                        .children_reactive(move || {
                            cursor.get().map(|_| {
                                container()
                                    .layout(LayoutStyle::flex_column().fixed_size(20.0, 20.0))
                                    .drop_target(nested.clone(), |_| true, |_| {})
                            })
                        }),
                ),
        );
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        route(&mut router, &mut cx, &layout, pointer(PointerPhase::Down, 10.0, 10.0));
        route(&mut router, &mut cx, &layout, pointer(PointerPhase::Move, 30.0, 25.0));
        route(&mut router, &mut cx, &layout, pointer(PointerPhase::Move, 150.0, 20.0));
        assert!(cursor.peek().is_some(), "hover reached the target");
        route(&mut router, &mut cx, &layout, pointer(PointerPhase::Up, 150.0, 20.0));
    });
}
