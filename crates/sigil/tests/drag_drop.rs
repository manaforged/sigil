#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::too_many_lines)]
#![allow(clippy::shadow_unrelated)]

use std::cell::RefCell;
use std::rc::Rc;

use sigil::prelude::*;
use sigil_hal::{Point, PointerEventInit};
use sigil_renderer::{SceneCommand, emit_scene};

const WIDTH: f32 = 640.0;
const HEIGHT: f32 = 480.0;

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(WIDTH, HEIGHT))
        .expect("layout");
    layout
}

fn rect_colors(cx: &Cx, root: NodeId) -> Vec<sigil_style::Rgba8> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::Rect(r) => Some(r.color),
            _ => None,
        })
        .collect()
}

fn glyph_texts(cx: &Cx, root: NodeId) -> Vec<String> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::GlyphRun(d) => Some(d.run.text.clone()),
            _ => None,
        })
        .collect()
}

const fn pointer(phase: PointerPhase, x: f32, y: f32) -> ShellEvent {
    let (button, buttons) = match phase {
        PointerPhase::Down => (Some(MouseButton::Left), MouseButtons::LEFT),
        PointerPhase::Move => (None, MouseButtons::LEFT),
        _ => (None, MouseButtons::empty()),
    };
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

const fn right_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Right),
        buttons: MouseButtons::RIGHT,
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Down,
    }))
}

fn route(router: &mut EventRouter, cx: &mut Cx, layout: &LayoutEngine, event: ShellEvent) {
    router.route(cx, layout, event);
}

type Drops = Rc<RefCell<Vec<String>>>;
type Downs = Rc<RefCell<u32>>;

fn demo_tree(
    state: DragState<String>,
    drops: &Drops,
    downs: &Downs,
    accepts: impl Fn(&String) -> bool + 'static,
) -> impl IntoNode {
    let on_drop = {
        let drops = Rc::clone(drops);
        move |payload: String| drops.borrow_mut().push(payload)
    };
    let count_down = {
        let downs = Rc::clone(downs);
        move |_: &mut sigil_hal::PointerEvent| *downs.borrow_mut() += 1
    };
    container()
        .layout(LayoutStyle::flex_row().fixed_size(WIDTH, HEIGHT))
        .child(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                .drag_source(state.clone(), "task-7".to_owned())
                .pointer_down(count_down)
                .child(text("task-7")),
        )
        .child(container().layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0)))
        .child(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(100.0, 40.0))
                .drop_target(state.clone(), accepts, on_drop),
        )
        .child(drag_ghost(state, |payload| {
            text(format!("moving {payload}"))
        }))
}

fn demo() -> (DragState<String>, Drops, Downs, Cx, NodeId) {
    let state = DragState::<String>::new();
    let drops: Drops = Rc::new(RefCell::new(Vec::new()));
    let downs: Downs = Rc::new(RefCell::new(0_u32));
    let mut cx = Cx::new_root();
    let root = cx.add(demo_tree(state.clone(), &drops, &downs, |_| true));
    (state, drops, downs, cx, root)
}

#[test]
fn plain_click_and_right_click_never_start_a_session() {
    sigil::reactive::Runtime::scope("drag-drop-click", || {
        let (state, drops, downs, mut cx, root) = demo();
        let layout = compute(&mut cx, root);
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
        let layout = compute(&mut cx, root);
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
        let layout = compute(&mut cx, root);
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

#[test]
fn target_highlight_is_reactive_and_clears_on_move_away() {
    sigil::reactive::Runtime::scope("drag-drop-highlight", || {
        let (_state, drops, _downs, mut cx, root) = demo();
        let layout = compute(&mut cx, root);
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
        let layout = compute(&mut cx, root);
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
        let layout = compute(&mut cx, root);
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
        let layout = compute(&mut cx, root);
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
        let layout = compute(&mut cx, root);
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
        let layout = compute(&mut cx, root);
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
        let layout = compute(&mut cx, root);
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
        let _layout = compute(&mut cx, root);
        assert!(
            !glyph_texts(&cx, root).iter().any(|t| t == "moving task-7"),
            "after the drop: the ghost chip is gone",
        );
    });
}
