use crate::animation::Easing;
use crate::animation_runtime::AnimationConfig;
use crate::geometry::LogicalSize;
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;
use crate::layout::TextMeasure;
use crate::reactive::Runtime;
use crate::style::NumericAtomKind;
use std::cell::RefCell;
use std::rc::Rc;

use crate::animation_runtime::tick;
use crate::events::EventRouter;
use crate::events::{
    ModifiersState, MouseButton, MouseButtons, PointerEvent, PointerEventInit, PointerKind,
    PointerPhase, ShellEvent,
};
use crate::geometry::Point;
use crate::prelude::*;
use crate::style::Transform2D;

struct Measure;

impl TextMeasure for Measure {
    fn measure(&self, _content: &TextContent, _space: Size) -> LogicalSize {
        LogicalSize::default()
    }
}

fn layout(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(Measure));
    layout
        .compute(cx, root, Size::fixed(400.0, 300.0))
        .expect("layout computes");
    layout
}

fn pointer(phase: PointerPhase, x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
        phase,
    }))
}

fn transformed(child: NodeBuilder) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(400.0, 300.0))
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(100.0, 60.0))
                .transform(
                    Transform2D::IDENTITY
                        .translate(100.0, 40.0)
                        .scale(2.0, 2.0)
                        .origin(0.0, 0.0),
                )
                .child(child),
        )
}

#[test]
fn transformed_input_tracks_motion_without_rebuilding_the_layout_cache() {
    Runtime::scope("transformed-input-cache", || {
        let shift = signal(0.0);
        let positions = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&positions);
        let child = container()
            .layout(LayoutStyle::block().fixed_size(50.0, 20.0))
            .animate(NumericAtomKind::TranslateX, shift)
            .pointer_down(move |event| sink.borrow_mut().push(event.position));
        let mut cx = Cx::new_root();
        let root = cx.add(transformed(child));
        let layout = layout(&mut cx, root);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer(PointerPhase::Down, 110.0, 50.0));
        assert_eq!(&*positions.borrow(), &[Point::new(5.0, 5.0)]);
        shift.set(30.0);
        router.route(&mut cx, &layout, pointer(PointerPhase::Down, 110.0, 50.0));
        assert_eq!(positions.borrow().len(), 1);
        router.route(&mut cx, &layout, pointer(PointerPhase::Down, 170.0, 50.0));
        assert_eq!(
            &*positions.borrow(),
            &[Point::new(5.0, 5.0), Point::new(5.0, 5.0)]
        );
    });
}

#[test]
fn captured_pointer_moves_keep_local_coordinates_outside_a_transformed_control() {
    Runtime::scope("transformed-pointer-capture", || {
        let positions = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&positions);
        let child = container()
            .layout(LayoutStyle::block().fixed_size(50.0, 20.0))
            .pointer_down(PointerEvent::capture_pointer)
            .pointer_move(move |event| {
                if event.is_captured() {
                    sink.borrow_mut().push(event.position);
                }
            });
        let mut cx = Cx::new_root();
        let root = cx.add(transformed(child));
        let layout = layout(&mut cx, root);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer(PointerPhase::Down, 110.0, 50.0));
        router.route(&mut cx, &layout, pointer(PointerPhase::Move, 350.0, 90.0));
        assert_eq!(&*positions.borrow(), &[Point::new(125.0, 25.0)]);
    });
}

fn present_row(shown: Signal<bool>, clicks: Signal<i32>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column())
        .child(
            container()
                .layout(LayoutStyle::block().fixed_size(100.0, 40.0))
                .focusable(true)
                .presence(
                    shown,
                    AnimationConfig::Tween {
                        ms: 200,
                        easing: Easing::Linear,
                    },
                )
                .pointer_down(move |_| clicks.set(clicks.peek() + 1)),
        )
        .child(container().layout(LayoutStyle::block().fixed_size(100.0, 10.0)))
}

#[test]
fn exiting_presence_blocks_input_and_accessibility_before_releasing_layout_space() {
    let observed = Runtime::scope("presence-exit", || {
        let shown = signal(true);
        let clicks = signal(0);
        let mut cx = Cx::new_root();
        let root = cx.add(present_row(shown, clicks));
        let target = cx.nth_child(root, 0).expect("exiting row");
        let sibling = cx.nth_child(root, 1).expect("following row");
        tick(0.25);
        let mut layout = layout(&mut cx, root);
        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer(PointerPhase::Down, 10.0, 10.0));
        let initial_clicks = clicks.peek();
        shown.set(false);
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 300.0))
            .expect("exit layout");
        let during = layout
            .result(sibling)
            .expect("following row layout")
            .rect()
            .origin
            .y;
        router.route(&mut cx, &layout, pointer(PointerPhase::Down, 10.0, 10.0));
        let after_clicks = clicks.peek();
        let focusable = router.is_focus_target(&cx, &layout, target);
        let hidden = crate::a11y::node(&cx, target, &|_| None).is_hidden();
        tick(0.25);
        layout
            .compute(&mut cx, root, Size::fixed(400.0, 300.0))
            .expect("settled layout");
        let after = layout
            .result(sibling)
            .expect("following row layout")
            .rect()
            .origin
            .y;
        (
            initial_clicks,
            after_clicks,
            focusable,
            hidden,
            during.to_bits(),
            after.to_bits(),
        )
    });
    let expected = (1, 1, false, true, 40.0_f32.to_bits(), 0.0_f32.to_bits());
    assert_eq!(observed, expected);
}
