use super::*;

#[test]
fn slider_click_jumps_to_position() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-click", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).length(SLIDER_TRACK));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(90.0, 10.0));

        assert_approx(value.get(), 50.0);
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::Slider);
        assert_approx(node.a11y.value_now.unwrap().get(), 50.0);
        assert_eq!(node.a11y.value_min, Some(0.0));
        assert_eq!(node.a11y.value_max, Some(100.0));
    });
}

#[test]
fn slider_drag_updates_value_continuously() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-drag", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).length(SLIDER_TRACK));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(0.0, 10.0));
        router.route(&mut cx, &layout, pointer_move(135.0, 10.0));

        assert_approx(value.get(), 75.0);
    });
}

#[test]
fn slider_drag_with_step_quantizes_value() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-step", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).step(10.0).length(SLIDER_TRACK));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(47.0, 10.0));
        router.route(&mut cx, &layout, pointer_move(64.0, 10.0));

        assert_approx(value.get(), 40.0);
    });
}

#[test]
fn slider_keyboard_stepping() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-keyboard", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).step(5.0).length(SLIDER_TRACK));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            arrow_down(Key::ArrowRight, PhysicalKey::ArrowRight),
        );
        assert_approx(value.get(), 55.0);

        router.route(
            &mut cx,
            &layout,
            arrow_down(Key::ArrowLeft, PhysicalKey::ArrowLeft),
        );
        assert_approx(value.get(), 50.0);
    });
}

#[test]
fn slider_disabled_ignores_input() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-disabled", || {
        let value = signal(25.0_f32);
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(
            slider(value, 0.0..=100.0)
                .disabled(disabled)
                .step(5.0)
                .length(SLIDER_TRACK),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(180.0, 10.0));
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            arrow_down(Key::ArrowRight, PhysicalKey::ArrowRight),
        );

        assert_approx(value.get(), 25.0);
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
        assert!(!node.a11y.hittable);
    });
}

#[test]
fn slider_capture_releases_on_pointer_up() {
    sigil::reactive::Runtime::scope("sigil-widget-slider-capture-release", || {
        let value = signal(0.0_f32);
        let changes = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(slider(value, 0.0..=100.0).length(SLIDER_TRACK).change({
            let changes = Rc::clone(&changes);
            move |_| changes.set(changes.get() + 1)
        }));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(320.0, 120.0))
            .expect("layout computes");

        let mut router = EventRouter::new();
        router.route(&mut cx, &layout, pointer_down(0.0, 10.0));
        router.route(&mut cx, &layout, pointer_move(180.0, 10.0));
        router.route(&mut cx, &layout, pointer_up(180.0, 10.0));
        assert_approx(value.get(), 100.0);
        assert_eq!(changes.get(), 1);

        router.route(&mut cx, &layout, pointer_move(0.0, 10.0));
        assert_approx(value.get(), 100.0);
        assert_eq!(changes.get(), 1);
    });
}
